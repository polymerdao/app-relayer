// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title Cross-Chain Resolver Interface
 * @dev Standard interface for cross-chain execution resolution
 * 
 * This interface enables contracts to request and verify cross-chain executions
 * through a standardized pattern. It works in conjunction with relayers and
 * Polymer proof system to enable secure cross-chain communication.
 *
 * Key Features:
 * - Replay protection via nonces
 * - Event-based execution requests
 * - Payload encoding standardization
 *
 * Workflow:
 * 1. Relayer calls crossChainChecker() to check if execution is needed
 * 2. If canExec=true, relayer calls requestRemoteExecution()
 * 3. Contract emits CrossChainExecRequested event with payload
 * 4. Relayer captures event and creates proof
 * 5. Relayer submits proof to destination chain for execution
 */
interface ICrossChainResolver {
    /**
     * @notice Emitted when a cross-chain execution is requested
     * @param destinationChainId The ID of the destination chain where execution should occur
     * @param execPayload The ABI-encoded payload containing the function call data
     * @param nonce A unique identifier for this execution request to prevent replay attacks
     */
    event CrossChainExecRequested(
        uint32 indexed destinationChainId,
        bytes execPayload,
        uint256 indexed nonce
    );
    
    /**
     * @notice Checks if a cross-chain execution is needed and prepares the payload
     * @dev This function is called by relayers to determine if an execution should be triggered
     * @param destinationChainId The ID of the destination chain to check against
     * @return canExec True if execution should be triggered
     * @return execPayload The ABI-encoded payload containing the function call data
     * @return nonce A unique identifier for this execution request
     */
    function crossChainChecker(uint32 destinationChainId) 
        external 
        view 
        returns (bool canExec, bytes memory execPayload, uint256 nonce);
    
    /**
     * @notice Requests execution on a remote chain
     * @dev This function is called by relayers when crossChainChecker returns true
     * @param destinationChainId The ID of the destination chain where execution should occur
     * @notice Emits CrossChainExecRequested event with execution details
     */
    function requestRemoteExecution(uint32 destinationChainId) external;
}
