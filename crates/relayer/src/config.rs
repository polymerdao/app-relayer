// Config structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub resolver_address: String,
    pub state_sync_address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RelayerConfig {
    pub polling_interval_ms: u64,
    pub chains: Vec<ChainConfig>,
}

