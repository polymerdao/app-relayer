// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./StateSyncV2.sol";
import "./IResolver.sol";
import "./CrossChainExecutor.sol";

/**
 * @title BatchedStateSync
 * @dev Extends StateSyncV2 to support batched state synchronization using the resolver interface
 * for more efficient cross-chain state updates
 */
contract BatchedStateSync is StateSyncV2, ICrossChainResolver, CrossChainExecutor {
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

    /**
     * @dev Constructor to initialize the BatchedStateSync contract
     * @param _polymerProver Address of the Polymer prover contract
     * @param _batchThreshold Number of updates required before a batch is ready
     */
    constructor(address _polymerProver, uint256 _batchThreshold) 
        StateSyncV2(_polymerProver)
        CrossChainExecutor(_polymerProver)
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
            execPayload = abi.encodeWithSelector(this.receiveBatch.selector, pendingUpdateQueue);
            
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
        
        // Emit event for the relayer to pick up
        emit CrossChainExecRequested(
            destinationChainId,
            abi.encodeWithSelector(this.receiveBatch.selector, pendingUpdateQueue),
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
     * @dev Process a batch of state updates
     * @param updates Array of PendingUpdate structs containing the batched state changes
     */
    function receiveBatch(PendingUpdate[] memory updates) public {
        // Process each update in the batch
        for (uint256 i = 0; i < updates.length; i++) {
            PendingUpdate memory update = updates[i];
            
            // Apply each state update using StateSyncV2's functionality
            store[update.hashedKey] = update.value;
            keyVersions[update.hashedKey] = update.version;
            
            // Emit update event
            emit ValueUpdated(update.hashedKey, update.value, update.version);
        }
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

}
