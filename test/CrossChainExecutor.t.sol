pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/CrossChainExecutor.sol";

contract MockPolymerProver is ICrossL2ProverV2 {
    function validateEvent(bytes calldata proof) external pure override returns (
        uint32 chainId,
        address emittingContract,
        bytes memory topics,
        bytes memory data
    ) {
        return abi.decode(proof, (uint32, address, bytes, bytes));
    }
}

contract TestExecutor is CrossChainExecutor {
    uint256 public value;
    
    constructor(address prover) CrossChainExecutor(prover) {}
    
    function testFunction(uint256 newValue) public {
        value = newValue;
    }
}

contract CrossChainExecutorTest is Test {
    TestExecutor public executor;
    MockPolymerProver public mockProver;
    
    uint32 constant SOURCE_CHAIN_ID = 1;
    uint32 constant DEST_CHAIN_ID = 2;
    address constant SOURCE_CONTRACT = address(0x1234);

    function setUp() public {
        mockProver = new MockPolymerProver();
        executor = new TestExecutor(address(mockProver));
        vm.chainId(DEST_CHAIN_ID);
    }

    function testExecuteValidProof() public {
        bytes memory payload = abi.encodeWithSelector(TestExecutor.testFunction.selector, 42);
        bytes memory topics = abi.encode(
            executor.CROSS_CHAIN_EXEC_REQUESTED_SIG(),
            bytes32(uint256(DEST_CHAIN_ID)),
            bytes32(uint256(1))
        );
        
        bytes memory proof = abi.encode(
            SOURCE_CHAIN_ID,
            SOURCE_CONTRACT,
            topics,
            payload
        );

        vm.expectEmit(true, true, true, true);
        emit CrossChainExecuted(SOURCE_CHAIN_ID, SOURCE_CONTRACT, 1, bytes4(keccak256("testFunction(uint256)")), true);
        
        (bool success,) = executor.executeWithProof(proof);
        assertTrue(success);
        assertEq(executor.value(), 42);
        assertTrue(executor.usedNonces(SOURCE_CHAIN_ID, SOURCE_CONTRACT, 1));
    }

    function testReplayAttack() public {
        bytes memory proof = _createValidProof();
        executor.executeWithProof(proof);
        
        vm.expectRevert("Nonce already used");
        executor.executeWithProof(proof);
    }

    function testInvalidEventSignature() public {
        bytes memory invalidTopics = abi.encode(keccak256("WrongEvent()"), bytes32(0), bytes32(0));
        bytes memory proof = abi.encode(SOURCE_CHAIN_ID, SOURCE_CONTRACT, invalidTopics, "");
        
        vm.expectRevert("Invalid event signature");
        executor.executeWithProof(proof);
    }

    function testWrongDestinationChain() public {
        bytes memory topics = abi.encode(
            executor.CROSS_CHAIN_EXEC_REQUESTED_SIG(),
            bytes32(uint256(999)), // Wrong chain ID
            bytes32(uint256(1))
        );
        bytes memory proof = abi.encode(SOURCE_CHAIN_ID, SOURCE_CONTRACT, topics, "");
        
        vm.expectRevert("Invalid destination chain");
        executor.executeWithProof(proof);
    }

    function _createValidProof() internal view returns (bytes memory) {
        bytes memory topics = abi.encode(
            executor.CROSS_CHAIN_EXEC_REQUESTED_SIG(),
            bytes32(uint256(DEST_CHAIN_ID)),
            bytes32(uint256(1))
        );
        return abi.encode(SOURCE_CHAIN_ID, SOURCE_CONTRACT, topics, "");
    }
}
