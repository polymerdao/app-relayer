use serde::Serialize;

// Config structures
#[derive(Debug, Serialize, Clone)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub src_resolver_address: String,
    pub dest_dapp_address: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RelayerConfig {
    pub polling_interval_ms: u64,
    pub chains: Vec<ChainConfig>,
}

