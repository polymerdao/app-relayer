// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "../src/BatchedStateSync.sol";
import "../src/IResolver.sol";
import "../lib/prover-contracts/contracts/interfaces/ICrossL2ProverV2.sol";

contract MockPolymerProver is ICrossL2ProverV2 {
    function validateEvent(bytes calldata proof) external pure override returns (
        uint32 chainId,
        address emittingContract,
        bytes memory topics,
        bytes memory data
    ) {
        return abi.decode(proof, (uint32, address, bytes, bytes));
    }

    function inspectLogIdentifier(bytes calldata proof) 
        external 
        pure 
        override
        returns (uint32 srcChain, uint64 blockNumber, uint16 receiptIndex, uint8 logIndex) 
    {
        // Default mock implementation
        return (1, 1, 1, 1);
    }
    
    function inspectPolymerState(bytes calldata proof) 
        external 
        pure 
        override
        returns (bytes32 stateRoot, uint64 height, bytes memory signature) 
    {
        // Default mock implementation
        return (bytes32(0), 1, bytes(""));
    }
}

contract BatchedStateSyncTest is Test {
    BatchedStateSync public batchedSync;
    MockPolymerProver public mockProver;
    
    // Test constants
    uint256 constant BATCH_THRESHOLD = 3;
    uint32 constant SOURCE_CHAIN_ID = 1;
    uint32 constant DEST_CHAIN_ID = 2;
    
    function setUp() public {
        mockProver = new MockPolymerProver();
        batchedSync = new BatchedStateSync(address(mockProver), BATCH_THRESHOLD);
        
        // Set chain ID for this contract to simulate destination chain
        vm.chainId(DEST_CHAIN_ID);
    }
    
    // Test setValue and pending updates tracking
    function testSetValueAndPendingUpdates() public {
        // Set values and check if they're properly tracked in pending updates
        batchedSync.setBatchedValue("key1", "value1");
        batchedSync.setBatchedValue("key2", "value2");
        
        // Get the pending updates 
        BatchedStateSync.PendingUpdate[] memory updates = batchedSync.getPendingUpdates();
        
        // Verify the correct number of updates is tracked
        assertEq(updates.length, 2, "Should have 2 pending updates");
        assertEq(batchedSync.pendingUpdates(), 2, "Pending updates count should be 2");
        
        // Check if isPending mapping is correctly set
        bytes32 hashedKey1 = keccak256(abi.encodePacked(address(this), "key1"));
        bytes32 hashedKey2 = keccak256(abi.encodePacked(address(this), "key2"));
        
        assertTrue(batchedSync.isPending(hashedKey1), "Key1 should be marked as pending");
        assertTrue(batchedSync.isPending(hashedKey2), "Key2 should be marked as pending");
    }
    
    // Test crossChainChecker behavior
    function testCrossChainChecker() public {
        console.log("Testing crossChainChecker...");
        
        // Initially batch threshold not met
        (bool canExec, bytes memory execPayload, uint256 nonce) = batchedSync.crossChainChecker(DEST_CHAIN_ID);
        console.log("Initial check - canExec:", canExec);
        assertFalse(canExec, "Should not be ready to execute with no updates");
        
        // Add enough updates to trigger the batch threshold
        console.log("Adding updates...");
        batchedSync.setBatchedValue("key1", "value1");
        batchedSync.setBatchedValue("key2", "value2");
        batchedSync.setBatchedValue("key3", "value3");
        
        // Check pending updates
        uint256 pending = batchedSync.pendingUpdates();
        console.log("Pending updates after adding:", pending);
        
        // Now check again, should be ready to execute
        (canExec, execPayload, nonce) = batchedSync.crossChainChecker(DEST_CHAIN_ID);
        console.log("After threshold - canExec:", canExec, "nonce:", nonce);
        assertTrue(canExec, "Should be ready to execute after threshold reached");
        
        // Verify the execPayload is correctly formatted (contains the receiveBatch selector)
        bytes4 selector = bytes4(execPayload);
        console.logBytes4(selector);
        console.logBytes4(batchedSync.receiveBatch.selector);
        assertEq(selector, batchedSync.receiveBatch.selector, "Exec payload should contain receiveBatch selector");
        
        // Verify nonce is correct (should be 1 for first execution)
        assertEq(nonce, 1, "Nonce should be 1 for first execution");
    }
    
    // Test requestRemoteExecution
    function testRequestRemoteExecution() public {
        // Add enough updates to trigger the batch threshold
        batchedSync.setBatchedValue("key1", "value1");
        batchedSync.setBatchedValue("key2", "value2");
        batchedSync.setBatchedValue("key3", "value3");
        
        // Get pending updates *before* calling requestRemoteExecution to compare emitted payload
        BatchedStateSync.PendingUpdate[] memory updatesBefore = batchedSync.getPendingUpdates();
        bytes memory expectedPayload = abi.encodeWithSelector(batchedSync.receiveBatch.selector, updatesBefore);

        // Call requestRemoteExecution and capture the event
        vm.expectEmit(true, true, false, true); // Check destChainId (topic1), nonce (topic2), and payload (data)
        emit ICrossChainResolver.CrossChainExecRequested(DEST_CHAIN_ID, expectedPayload, 1);
        batchedSync.requestRemoteExecution(DEST_CHAIN_ID);
        
        // Verify pending queue is cleared
        BatchedStateSync.PendingUpdate[] memory updates = batchedSync.getPendingUpdates();
        assertEq(updates.length, 0, "Pending queue should be cleared");
        assertEq(batchedSync.pendingUpdates(), 0, "Pending updates count should be 0");
        
        // Verify isPending for keys is reset
        bytes32 hashedKey1 = keccak256(abi.encodePacked(address(this), "key1"));
        assertFalse(batchedSync.isPending(hashedKey1), "Key1 should not be pending anymore");
    }
    
    // Test executeWithProof and receiveBatch with mock proof
    function testExecuteWithProofAndReceiveBatch() public {
        // Create a source contract address
        address sourceContract = address(0x1234);
        
        // Create mock pending updates
        BatchedStateSync.PendingUpdate[] memory mockUpdates = new BatchedStateSync.PendingUpdate[](2);
        bytes32 hashedKey1 = keccak256(abi.encodePacked(sourceContract, "key1"));
        bytes32 hashedKey2 = keccak256(abi.encodePacked(sourceContract, "key2"));
        
        mockUpdates[0] = BatchedStateSync.PendingUpdate({
            hashedKey: hashedKey1,
            value: bytes("value1"),
            version: 1
        });
        
        mockUpdates[1] = BatchedStateSync.PendingUpdate({
            hashedKey: hashedKey2,
            value: bytes("value2"),
            version: 1
        });
        
        // Define nonce for this execution
        uint256 nonce = 1;

        // Prepare data for executeWithProof
        // 1. Calculate execPayload for the receiveBatch function
        bytes memory execPayload = abi.encodeWithSelector(batchedSync.receiveBatch.selector, mockUpdates);

        // 2. Prepare topics for CrossChainExecRequested event
        bytes32 eventSelector = batchedSync.CROSS_CHAIN_EXEC_REQUESTED_SIG(); // Inherited from CrossChainExecutor
        bytes32 destChainIdTopic = bytes32(uint256(DEST_CHAIN_ID));
        bytes32 nonceTopic = bytes32(nonce);
        bytes memory topics = abi.encodePacked(eventSelector, destChainIdTopic, nonceTopic);

        // 3. Create mock proof by encoding the parameters MockPolymerProver.validateEvent will return
        bytes memory mockProof = abi.encode(
            SOURCE_CHAIN_ID,    // Source chain ID
            sourceContract,     // Source contract
            topics,             // Event topics
            execPayload         // Encoded call to receiveBatch
        );
        
        // Call executeWithProof with the mock proof
        // This will decode the proof, check nonce, and call receiveBatch internally
        batchedSync.executeWithProof(mockProof);
        
        // Verify that the nonce is marked as used for this source chain and contract
        assertTrue(batchedSync.usedNonces(SOURCE_CHAIN_ID, sourceContract, nonce), "Nonce should be marked as used");
        
        // Verify state was updated
        bytes memory value1 = batchedSync.getValue(sourceContract, "key1");
        bytes memory value2 = batchedSync.getValue(sourceContract, "key2");
        
        assertEq(string(value1), "value1", "Value1 should be correctly set");
        assertEq(string(value2), "value2", "Value2 should be correctly set");
    }
    
    // Test setBatchThreshold
    function testSetBatchThreshold() public {
        // Check initial threshold
        assertEq(batchedSync.batchThreshold(), BATCH_THRESHOLD, "Initial threshold should match constructor value");
        
        // Set new threshold
        uint256 newThreshold = 5;
        batchedSync.setBatchThreshold(newThreshold);
        
        // Verify threshold was updated
        assertEq(batchedSync.batchThreshold(), newThreshold, "Threshold should be updated to new value");
        
        // Test that the new threshold is respected
        for (uint256 i = 0; i < newThreshold - 1; i++) {
            string memory key = string(abi.encodePacked("key", i));
            batchedSync.setBatchedValue(key, "value");
        }
        
        // Should not yet be ready to execute
        (bool canExec,,) = batchedSync.crossChainChecker(DEST_CHAIN_ID);
        assertFalse(canExec, "Should not be ready to execute with less than threshold updates");
        
        // Add one more update to meet threshold
        batchedSync.setBatchedValue("finalKey", "value");
        
        // Now should be ready to execute
        (canExec,,) = batchedSync.crossChainChecker(DEST_CHAIN_ID);
        assertTrue(canExec, "Should be ready to execute after meeting new threshold");
    }
}
