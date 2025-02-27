// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./StateSyncV2.sol";

/**
 * @title BatchedStateSync
 * @dev Extends StateSyncV2 to support batched state synchronization using the resolver interface
 * for more efficient cross-chain state updates
 */
contract BatchedStateSync is StateSyncV2 {
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
    {
        batchThreshold = _batchThreshold;
        pendingUpdates = 0;
    }
    
    /**
     * @dev Override the setValue function to track pending updates
     * @param key The key to set
     * @param value The value to associate with the key
     */
    function setValue(string calldata key, bytes calldata value) external override {
        // Call the parent implementation to perform the actual state update
        super.setValue(key, value);
        
        // Get the hashedKey
        bytes32 hashedKey = keccak256(abi.encodePacked(msg.sender, key));
        
        // Add to pending updates if not already pending
        if (!isPending[hashedKey]) {
            // Get the version that was just set
            uint256 version = getKeyVersionByHash(hashedKey);
            
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
     * @dev Resolver interface function that tells the relayer when to execute
     * @return canExec True if batch threshold is reached and updates should be processed
     * @return execPayload The encoded payload containing the batch of updates to process
     */
    function checker() 
        external 
        view 
        returns (bool canExec, bytes memory execPayload) 
    {
        // Check if we have reached the batch threshold
        if (pendingUpdates >= batchThreshold) {
            // Prepare the execution payload
            execPayload = abi.encodeWithSelector(
                this.processBatch.selector
            );
            
            return (true, execPayload);
        }
        
        return (false, "");
    }
    
    /**
     * @dev Process a batch of pending updates
     * @notice This function should be called by the relayer when checker returns true
     */
    function processBatch() external {
        require(pendingUpdates >= batchThreshold, "Batch threshold not reached");
        
        uint256 batchSize = pendingUpdateQueue.length;
        
        // Clear the pending updates
        for (uint256 i = 0; i < batchSize; i++) {
            isPending[pendingUpdateQueue[i].hashedKey] = false;
        }
        
        // Reset the state
        delete pendingUpdateQueue;
        pendingUpdates = 0;
        
        // Emit event to notify that a batch has been dispatched
        emit BatchDispatched(batchSize);
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
