// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./StateSyncV2.sol";
import "./IResolver.sol";

/**
 * @title BatchedStateSync
 * @dev Extends StateSyncV2 to support batched state synchronization using the resolver interface
 * for more efficient cross-chain state updates
 */
contract BatchedStateSync is StateSyncV2, ICrossChainResolver {
    // Configuration for batching
    uint256 public batchThreshold;
    
    // State variables for tracking batched updates
    uint256 public pendingUpdates;
    
    // Array to store pending updates
    struct PendingUpdate {
        bytes32 hashedKey;
        bytes value;
        uint256 version;
    }
    
    PendingUpdate[] public pendingUpdateQueue;

    // Track nonces for cross-chain requests
    uint256 private crossChainNonce;
    
    // Mapping to check if a key is already in the pending queue
    mapping(bytes32 => bool) public isPending;
    
    // Event for tracking when batches are dispatched
    event BatchDispatched(uint256 batchSize);

    // Event for tracking when cross-chain execution is received
    event CrossChainBatchReceived(uint32 sourceChainId, address sourceContract, uint256 nonce);
    
    /**
     * @dev Constructor to initialize the BatchedStateSync contract
     * @param _polymerProver Address of the Polymer prover contract
     * @param _batchThreshold Number of updates required before a batch is ready
     */
    constructor(address _polymerProver, uint256 _batchThreshold) 
        StateSyncV2(_polymerProver) 
    {
        batchThreshold = _batchThreshold;
        pendingUpdates = 0;
    }
    
    /**
     * @dev Set a value and track it for batching
     * @param key The key to set
     * @param value The value to associate with the key
     */
    function setBatchedValue(string calldata key, bytes calldata value) external {
        // Call the parent implementation to perform the actual state update
        this.setValue(key, value);
        
        // Get the hashedKey
        bytes32 hashedKey = keccak256(abi.encodePacked(msg.sender, key));
        
        // Add to pending updates if not already pending
        if (!isPending[hashedKey]) {
            // Get the version that was just set
            uint256 version = this.getKeyVersionByHash(hashedKey);
            
            pendingUpdateQueue.push(PendingUpdate({
                hashedKey: hashedKey,
                value: value,
                version: version
            }));
            
            isPending[hashedKey] = true;
            pendingUpdates++;
        }
    }

    /**
     * @dev Checks if a batch is ready for cross-chain execution
     * @param destinationChainId The chain ID where execution should happen
     * @return canExec Boolean indicating if execution should proceed
     * @return execPayload The payload to execute on destination chain
     * @return nonce A unique identifier to prevent replay attacks
     */
    function crossChainChecker(uint32 destinationChainId) 
        external 
        view 
        override
        returns (bool canExec, bytes memory execPayload, uint256 nonce) 
    {
        if (pendingUpdates >= batchThreshold) {
            execPayload = abi.encodeWithSelector(
                this.processBatch.selector
            );
            
            nonce = crossChainNonce + 1;
            return (true, execPayload, nonce);
        }
        
        return (false, "", 0);
    }
    
    /**
     * @dev Requests execution on a destination chain and clears the queue
     * @param destinationChainId The chain ID where execution should happen
     */
    function requestRemoteExecution(uint32 destinationChainId) external {
        (bool canExec, bytes memory execPayload, uint256 nonce) = this.crossChainChecker(destinationChainId);
        require(canExec, "Batch threshold not reached");
        
        // Increment nonce
        crossChainNonce++;
        
        // Pack the pending updates into the CrossChainExecRequested event data
        bytes memory batchData = abi.encode(pendingUpdateQueue);
        
        // Emit event for the relayer to pick up
        emit CrossChainExecRequested(
            destinationChainId,
            batchData,
            nonce
        );
        
        // Clear queue immediately on source chain
        uint256 batchSize = pendingUpdateQueue.length;
        for (uint256 i = 0; i < batchSize; i++) {
            isPending[pendingUpdateQueue[i].hashedKey] = false;
        }
        
        delete pendingUpdateQueue;
        pendingUpdates = 0;
    }   

    /**
     * @dev Process a cross-chain batch request
     * @param proof The Polymer proof of the cross-chain request
     */
    function processBatch(bytes calldata proof) external {
        // Validate the cross-chain proof
        (
            uint32 sourceChainId,
            address sourceContract,
            bytes memory topics,
            bytes memory data
        ) = polymerProver.validateEvent(proof);
        
        // Verify this is a CrossChainExecRequested event
        bytes32 expectedSelector = keccak256("CrossChainExecRequested(uint32,bytes,uint256)");
        require(extractEventSelector(topics) == expectedSelector, "Invalid event");
        
        // Verify this chain is the target
        uint32 targetChainId = extractDestinationChainId(topics);
        require(targetChainId == block.chainid, "Wrong destination chain");
        
        // Extract nonce and verify proof hasn't been used
        uint256 eventNonce = extractNonce(topics);
        bytes32 proofHash = keccak256(
            abi.encodePacked(sourceChainId, sourceContract, eventNonce)
        );
        
        require(!usedProofHashes[proofHash], "Proof already used");
        usedProofHashes[proofHash] = true;
        
        // Decode the data field which should contain the batch updates
        (PendingUpdate[] memory updates) = abi.decode(data, (PendingUpdate[]));
        
        // Process each update in the batch
        for (uint256 i = 0; i < updates.length; i++) {
            PendingUpdate memory update = updates[i];
            
            // Apply each state update using StateSyncV2's functionality
            // We need to use the low-level call since we're working with hashedKeys
            bytes memory storeValue = update.value;
            store[update.hashedKey] = storeValue;
            keyVersions[update.hashedKey] = update.version;
            
            // Emit update event
            emit ValueUpdated(update.hashedKey, update.value, update.version);
        }
        
        emit CrossChainBatchReceived(sourceChainId, sourceContract, eventNonce);
    }   

    /**
     * @dev Helper function to extract event selector from topics
     */
    function extractEventSelector(bytes memory topics) private pure returns (bytes32) {
        require(topics.length >= 32, "Invalid topics length");
        bytes32 selector;
        assembly {
            selector := mload(add(topics, 32))
        }
        return selector;
    }
    
    /**
     * @dev Helper function to extract destination chain ID from topics
     */
    function extractDestinationChainId(bytes memory topics) private pure returns (uint32) {
        require(topics.length >= 64, "Invalid topics length");
        bytes32 topicValue;
        assembly {
            topicValue := mload(add(topics, 64))
        }
        return uint32(uint256(topicValue));
    }
    
    /**
     * @dev Helper function to extract nonce from topics
     */
    function extractNonce(bytes memory topics) private pure returns (uint256) {
        require(topics.length >= 96, "Invalid topics length");
        bytes32 topicValue;
        assembly {
            topicValue := mload(add(topics, 96))
        }
        return uint256(topicValue);
    }   

    /**
     * @dev Update the batch threshold
     * @param _newThreshold New threshold for batching
     */
    function setBatchThreshold(uint256 _newThreshold) external {
        // In a production contract, you would add access control here
        batchThreshold = _newThreshold;
    }
    
    /**
     * @dev Get the current pending updates
     * @return The array of pending updates
     */
    function getPendingUpdates() external view returns (PendingUpdate[] memory) {
        return pendingUpdateQueue;
    }

    /**
     * @dev Public accessor function for the usedProofHashes mapping
     * @param proofHash The hash to check
     * @return Whether the proof hash has been used
     */
    function isProofUsed(bytes32 proofHash) public view returns (bool) {
        return usedProofHashes[proofHash];
    }
}
