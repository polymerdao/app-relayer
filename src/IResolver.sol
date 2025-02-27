// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title IResolver
 * @dev Interface for resolvers that determine when and how to execute cross-chain operations
 * This interface is inspired by Gelato's resolver pattern
 */
interface IResolver {
    /**
     * @dev Checks whether a cross-chain operation should be executed and what payload to use
     * @return canExec Boolean indicating whether execution should proceed
     * @return execPayload The encoded function call to execute if canExec is true
     *
     * The relayer will periodically call this function to determine:
     * 1. IF a cross-chain message should be processed (canExec)
     * 2. WHAT the payload of that message should be (execPayload)
     */
    function checker() 
        external 
        view 
        returns (bool canExec, bytes memory execPayload);
}
