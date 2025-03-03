use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use relayer::{ChainConfig, RelayerApp, RelayerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting cross-chain relayer");

    // Load configuration
    let config = RelayerConfig {
        polling_interval_ms: 10000,
        chains: vec![
            ChainConfig {
                name: "Optimism Sepolia".to_string(),
                chain_id: 11155420,
                rpc_url: "https://optimism-sepolia.example.com".to_string(),
                resolver_address: "0x1234567890123456789012345678901234567890".to_string(),
                state_sync_address: "0x0987654321098765432109876543210987654321".to_string(),
            },
            ChainConfig {
                name: "Base Sepolia".to_string(),
                chain_id: 84532,
                rpc_url: "https://base-sepolia.example.com".to_string(),
                resolver_address: "0x2345678901234567890123456789012345678901".to_string(),
                state_sync_address: "0x9876543210987654321098765432109876543210".to_string(),
            },
        ],
    };

    // Private key (would come from env or secure storage)
    let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    // Create and run the application
    let mut app = RelayerApp::new(config, private_key);
    app.run().await
}
