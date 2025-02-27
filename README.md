# Cross-Chain Resolver Interface

The Cross-Chain Resolver interface provides a standardized way for smart contracts to request and verify executions across different blockchains.

## Core Components

### ICrossChainResolver Interface

```solidity
interface ICrossChainResolver {
    event CrossChainExecRequested(
        uint32 indexed destinationChainId,
        bytes execPayload,
        uint256 indexed nonce
    );
    
    function crossChainChecker(uint32 destinationChainId) 
        external 
        view 
        returns (bool canExec, bytes memory execPayload, uint256 nonce);
    
    function requestRemoteExecution(uint32 destinationChainId) external;
}
```

## How It Works

1. **Check Condition**: A relayer calls `crossChainChecker()` on the source chain, specifying the destination chain ID.

2. **Request Execution**: If execution conditions are met, the relayer calls `requestRemoteExecution()`, which:
   - Generates a unique nonce
   - Emits a `CrossChainExecRequested` event containing execution details

3. **Generate Proof**: The relayer captures the event and creates a cryptographic proof using Polymer.

4. **Execute on Destination**: The relayer submits this proof to the destination chain, which:
   - Verifies the proof is valid and from the correct source
   - Ensures the proof hasn't been used before (replay protection)
   - Executes the requested function with the provided payload

## Benefits

- **Security**: Cryptographic verification ensures only authorized executions occur
- **Deterministic**: Clear, verifiable process for cross-chain communication
- **Flexible**: Works with various underlying cross-chain messaging protocols
- **Replay-Protected**: Nonce system prevents duplicate executions
- **Chain-Specific**: Supports different execution parameters per destination chain

## Implementation Example

TODO
