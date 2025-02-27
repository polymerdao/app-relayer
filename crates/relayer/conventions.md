# Developer Conventions

## Logging
- Use the `tracing` crate for all logging
- Implement structured logging
- Log context-rich information (chain IDs, transaction hashes, etc.)
- Log levels:
  - `error!`: Failures requiring immediate attention
  - `warn!`: Potential issues that don't block operation
  - `info!`: Normal operational information
  - `debug!`: Detailed information for troubleshooting
  - `trace!`: Very detailed debugging information

Example:
```rust
info!(
    chain_id = chain.chain_id,
    chain_name = %chain.name,
    "Checking chain for pending operations"
);
```

## Instrumentation
- Add `#[instrument]` to functions that:
  - Make network calls (RPC interactions)
  - Access external services (Polymer API)
  - Perform long-running operations
  - Are entry points for key workflows

Example:
```rust
#[instrument(skip(self), fields(chain_id = chain.chain_id, chain_name = %chain.name))]
async fn check_chain(&self, chain: &ChainConfig) -> Result<()> {
    // Function implementation
}
```

## Error Handling
- Use `anyhow` for error propagation and `thiserror` for defining error types
- Create domain-specific error enums with the `thiserror` crate
- Use context with `anyhow::Context` to add relevant information to errors
- Propagate low-level errors upward, enrich with context at each layer

Example:
```rust
#[derive(Debug, thiserror::Error)]
pub enum RelayerError {
    #[error("Failed to connect to RPC endpoint for chain {chain_id}: {source}")]
    RpcConnection { chain_id: u64, source: anyhow::Error },
    
    #[error("Proof verification failed: {0}")]
    ProofVerification(String),
}

// Usage with context
let provider = Provider::<Http>::try_from(&chain.rpc_url)
    .context(format!("Failed to create provider for {}", chain.name))?;
```

## Async Operations
- Use `tokio` for all async operations
- Prefer `spawn` for short-lived tasks
- Use `spawn_blocking` for CPU-intensive operations
- Implement timeout handling for external API calls
- Use channels for communication between async tasks

Example:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Application startup

    // For concurrent operations
    let handles = join_all(futures).await;
    
    // For timeouts
    let result = tokio::time::timeout(
        Duration::from_secs(30), 
        self.request_proof(source_chain_id, dest_chain_id, tx_hash)
    ).await??;
}
```
