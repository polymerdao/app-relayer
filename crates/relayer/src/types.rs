use ethers::core::types::{Bytes, H256};
use std::sync::Arc;

// Re-export the config types
pub use crate::config::ChainConfig;

// Event detected by the event generator
#[derive(Debug, Clone)]
pub struct RelayEvent {
    pub source_chain: ChainConfig,
    pub source_resolver_address: String,
    pub destination_chain: ChainConfig,
    pub dest_dapp_address: String,
    pub exec_payload: Bytes,
    pub nonce: u64,
    pub meta: EventMeta,
}

#[derive(Debug, Clone)]
pub struct EventMeta {
    pub tx_hash: Option<H256>,
    pub block_number: u64,
    pub tx_index: u32,
    pub log_index: u32,
}

// Proof request sent to the proof fetcher
#[derive(Debug, Clone)]
pub struct ProofRequest {
    pub event: RelayEvent,
    pub tx_hash: H256,
    pub destination_chain_id: u64,
    pub dest_contract_address: String,
}

// Delivery request sent to the event deliverer
#[derive(Debug, Clone)]
pub struct DeliveryRequest {
    pub destination_chain_id: u64,
    pub destination_contract_address: String,
    pub event: RelayEvent,
    pub proof: Bytes,
}

// Define error types
#[derive(Debug, thiserror::Error)]
pub enum RelayerError {
    #[error("Failed to connect to RPC endpoint for chain {chain_id}: {source}")]
    RpcConnection {
        chain_id: u64,
        source: anyhow::Error,
    },

    #[error("Transaction failed on chain {chain_id}: {source}")]
    TransactionFailed {
        chain_id: u64,
        source: anyhow::Error,
    },

    #[error("Proof verification failed: {0}")]
    ProofVerification(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Resolver error: {0}")]
    ResolverError(String),
}
