use ethers::core::types::{Bytes, H256};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Re-export the config types
pub use crate::config::{ChainConfig, RelayerConfig};

// Event detected by the event generator
#[derive(Debug, Clone)]
pub struct RelayEvent {
    pub source_chain: Arc<ChainConfig>,
    pub destination_chain: Arc<ChainConfig>,
    pub exec_payload: Bytes,
    pub tx_hash: Option<H256>,
    pub nonce: u64,
}

// Proof request sent to the proof fetcher
#[derive(Debug, Clone)]
pub struct ProofRequest {
    pub event: RelayEvent,
    pub tx_hash: H256,
}

// Delivery request sent to the event deliverer
#[derive(Debug, Clone)]
pub struct DeliveryRequest {
    pub event: RelayEvent,
    pub proof: Bytes,
}

// Define error types
#[derive(Debug, thiserror::Error)]
pub enum RelayerError {
    #[error("Failed to connect to RPC endpoint for chain {chain_id}: {source}")]
    RpcConnection { chain_id: u64, source: anyhow::Error },
    
    #[error("Transaction failed on chain {chain_id}: {source}")]
    TransactionFailed { chain_id: u64, source: anyhow::Error },
    
    #[error("Proof verification failed: {0}")]
    ProofVerification(String),
    
    #[error("Channel error: {0}")]
    ChannelError(String),
    
    #[error("Resolver error: {0}")]
    ResolverError(String),
}
