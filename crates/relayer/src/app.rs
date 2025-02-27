use std::{sync::Arc, time::Duration};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{info, error, instrument};

use crate::{
    RelayerConfig, ChainConfig,
    EventGenerator, ProofFetcher, EventDeliverer,
};

pub struct RelayerApp {
    event_generator: EventGenerator,
    proof_fetcher: ProofFetcher,
    event_deliverer: EventDeliverer,
}

impl RelayerApp {
    #[instrument(skip_all, fields(config.chains_count = config.chains.len()))]
    pub fn new(config: RelayerConfig, private_key: &str) -> Self {
        info!("Initializing relayer application");
        
        // Convert chains to Arc for sharing
        let chains: Vec<Arc<ChainConfig>> = config.chains.iter()
            .map(|c| Arc::new(c.clone()))
            .collect();
        
        // Create channels for communication between components
        let (event_tx, event_rx) = mpsc::channel(100);
        let (delivery_tx, delivery_rx) = mpsc::channel(100);
        
        // Create components
        let event_generator = EventGenerator::new(
            chains.clone(),
            private_key.to_string(),
            Duration::from_millis(config.polling_interval_ms),
            event_tx,
        );
        
        let proof_fetcher = ProofFetcher::new(
            event_rx,
            delivery_tx,
            "https://api.polymer.zone/v1/proofs".to_string(),
        );
        
        let event_deliverer = EventDeliverer::new(
            private_key.to_string(),
            delivery_rx,
        );
        
        Self {
            event_generator,
            proof_fetcher,
            event_deliverer,
        }
    }

    /// Start all relayer components and wait for completion
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        info!("Starting all relayer components");
        
        // Start components in separate tasks
        let generator_handle = tokio::spawn(async move {
            if let Err(e) = self.event_generator.start().await {
                error!(error = %e, "Event generator error");
            }
        });
        
        let fetcher_handle = tokio::spawn(async move {
            if let Err(e) = self.proof_fetcher.start().await {
                error!(error = %e, "Proof fetcher error");
            }
        });
        
        let deliverer_handle = tokio::spawn(async move {
            if let Err(e) = self.event_deliverer.start().await {
                error!(error = %e, "Event deliverer error");
            }
        });
        
        tokio::select! {
            _ = generator_handle => error!("Event generator task exited"),
            _ = fetcher_handle => error!("Proof fetcher task exited"),
            _ = deliverer_handle => error!("Event deliverer task exited"),
        }
        
        Ok(())
    }
}
