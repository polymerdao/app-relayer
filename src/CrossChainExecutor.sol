// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

import "./ICrossL2ProverV2.sol";

/**
 * @title CrossChainExecutor
 * @dev Base contract for executing cross-chain function calls
 * 
 * This contract decodes proofs from Polymer's cross-chain prover and executes
 * the requested function calls on the destination chain. It ensures that only
 * authorized cross-chain messages from verified source contracts are executed.
 * 
 * Usage:
 * 1. Contract on destination chain inherits from CrossChainExecutor
 * 2. Contract implements any business logic functions that can be called cross-chain
 * 3. Relayer submits proof to executeWithProof() method
 * 4. CrossChainExecutor verifies and executes the requested function
 */
abstract contract CrossChainExecutor {
    // Polymer's cross-chain prover contract
    ICrossL2ProverV2 public immutable prover;
    
    // Mapping to track used nonces for replay protection
    mapping(uint32 => mapping(address => mapping(uint256 => bool))) public usedNonces;
    
    // Event emitted when a cross-chain execution is processed
    event CrossChainExecuted(
        uint32 sourceChainId,
        address sourceContract,
        uint256 nonce,
        bytes4 selector,
        bool success
    );
    
    // Event signature for CrossChainExecRequested
    bytes32 private constant CROSS_CHAIN_EXEC_REQUESTED_SIG = 
        keccak256("CrossChainExecRequested(uint32,bytes,uint256)");
    
    /**
     * @dev Constructor sets the Polymer prover contract address
     * @param _prover Address of the ICrossL2ProverV2 implementation
     */
    constructor(address _prover) {
        require(_prover != address(0), "Invalid prover address");
        prover = ICrossL2ProverV2(_prover);
    }
    
    /**
     * @notice Executes a function call from a cross-chain request
     * @dev Verifies the proof, decodes the event data, and executes the requested function
     * @param proof The Polymer proof containing the cross-chain execution request
     * @return success Whether the execution was successful
     * @return result The return data from the function call
     */
    function executeWithProof(bytes calldata proof) 
        external 
        returns (bool success, bytes memory result) 
    {
        // Validate the event using Polymer prover
        (
            uint32 sourceChainId,
            address sourceContract,
            bytes memory topics,
            bytes memory unindexedData
        ) = prover.validateEvent(proof);
        
        // Verify this is a CrossChainExecRequested event
        bytes32 eventSig = abi.decode(topics, (bytes32));
        require(eventSig == CROSS_CHAIN_EXEC_REQUESTED_SIG, "Invalid event signature");
        
        // Decode the event data
        // Topics format: [eventSig, destinationChainId, nonce]
        bytes memory topicsData = topics;
        assembly {
            // Skip the first 32 bytes (event signature)
            topicsData := add(topicsData, 32)
        }
        
        // Parse indexed parameters
        (uint32 destinationChainId, uint256 nonce) = abi.decode(topicsData, (uint32, uint256));
        
        // Parse unindexed parameters
        bytes memory execPayload = abi.decode(unindexedData, (bytes));
        
        // Verify destination chain matches current chain
        require(destinationChainId == getChainId(), "Invalid destination chain");
        
        // Prevent replay attacks
        require(!usedNonces[sourceChainId][sourceContract][nonce], "Nonce already used");
        usedNonces[sourceChainId][sourceContract][nonce] = true;
        
        // Extract function selector and parameters
        bytes4 selector;
        assembly {
            selector := mload(add(execPayload, 32))
        }
        
        // Execute the function call
        (success, result) = address(this).call(execPayload);
        
        // Emit execution event
        emit CrossChainExecuted(
            sourceChainId,
            sourceContract,
            nonce,
            selector,
            success
        );
        
        return (success, result);
    }
    
    /**
     * @dev Gets the current chain ID
     * @return The chain ID of the current blockchain
     */
    function getChainId() public view returns (uint32) {
        uint256 chainId;
        assembly {
            chainId := chainid()
        }
        return uint32(chainId);
    }
}
