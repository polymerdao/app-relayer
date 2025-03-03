use serde::Serialize;
use std::collections::HashMap;

// Chain configuration
#[derive(Debug, Serialize, Clone)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
}

// Source-destination pair for relaying
#[derive(Debug, Serialize, Clone)]
pub struct RelayPair {
    pub source_chain_id: u64,
    pub source_resolver_address: String,
    pub dest_chain_id: u64,
    pub dest_dapp_address: String,
}

// Main configuration structure
#[derive(Debug, Serialize, Clone)]
pub struct RelayerConfig {
    pub polling_interval_ms: u64,
    pub chains: HashMap<u64, ChainConfig>,
    pub relay_pairs: Vec<RelayPair>,
}

