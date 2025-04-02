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

### CrossChainExecutor Contract

```solidity
abstract contract CrossChainExecutor {
    // Executes a function call from a cross-chain request
    function executeWithProof(bytes calldata proof) 
        external 
        returns (bool success, bytes memory result);
        
    // Other functions and state variables...
}
```

## How It Works

1. **Check Condition**: A relayer calls `crossChainChecker()` on the source chain, specifying the destination chain ID.

2. **Request Execution**: If execution conditions are met, the relayer calls `requestRemoteExecution()`, which:
   - Generates a unique nonce
   - Emits a `CrossChainExecRequested` event containing execution details

3. **Generate Proof**: The relayer captures the event and creates a cryptographic proof using Polymer.

4. **Execute on Destination**: The relayer submits this proof to the destination chain's executor contract, which:
   - Verifies the proof is valid and from the correct source
   - Ensures the proof hasn't been used before (replay protection)
   - Executes the requested function with the provided payload

## Implementation Requirements

1. **Source Chain**: Implement the `ICrossChainResolver` interface on the source chain contract.

2. **Destination Chain**: Contracts that want to receive cross-chain executions **must** inherit from the `CrossChainExecutor` abstract contract.

3. **Proof Verification**: The destination contract uses Polymer's `ICrossL2ProverV2` interface to validate proofs.

4. **Execution Flow**:
   - The source contract emits a standardized event
   - The relayer captures this event and generates a proof
   - The relayer calls `executeWithProof()` on the destination contract
   - The executor validates the proof and calls the requested function on itself

## Benefits

- **Security**: Cryptographic verification ensures only authorized executions occur
- **Deterministic**: Clear, verifiable process for cross-chain communication
- **Flexible**: Works with various underlying cross-chain messaging protocols
- **Replay-Protected**: Nonce system prevents duplicate executions
- **Chain-Specific**: Supports different execution parameters per destination chain

## Setup Guide for Running the Relayer

### Prerequisites

- Docker installed on your machine
- A valid Polymer API key for Testnet
- Private key for transaction signing

### Step 1: Obtain a Polymer API Key

1. Visit [https://accounts.testnet.polymer.zone/manage-keys](https://accounts.testnet.polymer.zone/manage-keys) to create an API key
   - If you don't have permission, contact the Polymer team to provide one
2. Save this key for later use as `POLYMER_API_TOKEN_TESTNET`

### Step 2: Set Up Environment Variables

Create a file to store your environment variables or set them directly:

```bash
export PRIVATE_KEY=your_private_key_here
export POLYMER_API_TOKEN_TESTNET=your_polymer_api_token_here
export OPTIMISM_SEPOLIA_RPC_URL=https://sepolia.optimism.io  # Default value in justfile
```

### Step 3: Configure the Relayer

The configuration is specified in YAML files:
- For development: `./ts-relayer/config/config.dev.yaml`
- For testnet: `./ts-relayer/config/config.testnet.yaml`

The testnet configuration is set up to relay messages between the batch state sync contracts on Optimism Sepolia and Base Sepolia.

### Step 4: Build the Docker Image

```bash
just build-docker
```

This command builds the Docker image for the TypeScript relayer.

### Step 5: Run the Relayer

For local development:
```bash
just run
```

For testnet environment:
```bash
just run-docker
```

This will start the relayer in a Docker container with the appropriate configuration, using the proof-api endpoint: `https://proof.testnet.polymer.zone`.

### Step 6: Interact with the Contracts

To update batch values on the source chain (Optimism Sepolia):
```bash
just update-batch-testnet
```

To check if there are pending executions to be relayed:
```bash
just call-crossChainChecker-optimism-sepolia
just call-crossChainChecker-base-sepolia
```

## Deploying Contracts

To deploy the BatchedStateSync contract:

### On Development Chains:
```bash
just deploy-dev-chain-a  # Deploy to local chain A (port 8553)
just deploy-dev-chain-b  # Deploy to local chain B (port 8554)
```

### On Test Networks:
```bash
just deploy-optimism-sepolia  # Deploy to Optimism Sepolia
just deploy-base-sepolia      # Deploy to Base Sepolia
```

## Implementation Example

The `BatchedStateSync.sol` contract demonstrates how to implement the cross-chain resolver interface for batched state updates. This example shows:

- Batching multiple state updates before cross-chain execution
- Using a threshold to determine when to trigger execution
- Managing pending updates in a queue
- Handling cross-chain proof verification

Note: This is a demonstration implementation and should be thoroughly reviewed and tested before use in production.
