use anyhow::Result;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

use crate::{ChainConfig, EventDeliverer, EventGenerator, ProofFetcher, RelayerConfig};

pub struct RelayerApp {
    event_generator: Option<EventGenerator>,
    proof_fetcher: Option<ProofFetcher>,
    event_deliverer: Option<EventDeliverer>,
}

impl RelayerApp {
    #[instrument(skip_all, fields(config.chains_count = config.chains.len()))]
    pub fn new(config: RelayerConfig, private_key: &str) -> Self {
        info!("Initializing relayer application");

        // Convert chains to Arc for sharing
        let chains: Vec<Arc<ChainConfig>> =
            config.chains.iter().map(|c| Arc::new(c.clone())).collect();

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
            "your-api-token".to_string(), // TODO: Get this from config/env
        );

        let event_deliverer = EventDeliverer::new(private_key.to_string(), delivery_rx);

        Self {
            event_generator: Some(event_generator),
            proof_fetcher: Some(proof_fetcher),
            event_deliverer: Some(event_deliverer),
        }
    }

    /// Start all relayer components and wait for completion
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting all relayer components");

        // Take ownership of components
        let event_generator = self
            .event_generator
            .take()
            .expect("event_generator should not be empty");
        let mut proof_fetcher = self
            .proof_fetcher
            .take()
            .expect("proof_fetcher should not be empty");
        let mut event_deliverer = self
            .event_deliverer
            .take()
            .expect("event_deliverer should not be empty");

        // Start components in separate tasks
        let generator_handle = tokio::spawn(async move {
            if let Err(e) = event_generator.start().await {
                error!(error = %e, "Event generator error");
            }
        });

        let fetcher_handle = tokio::spawn(async move {
            if let Err(e) = proof_fetcher.start().await {
                error!(error = %e, "Proof fetcher error");
            }
        });

        let deliverer_handle = tokio::spawn(async move {
            if let Err(e) = event_deliverer.start().await {
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
