mod client;

use crate::types::{DeliveryRequest, ProofRequest, RelayEvent};
use anyhow::Result;
use ethers::core::types::Bytes;
use std::time::Duration;
use tokio::{sync::mpsc, time};
use tracing::{error, info, instrument};

pub struct ProofFetcher {
    event_rx: mpsc::Receiver<RelayEvent>,
    delivery_tx: mpsc::Sender<DeliveryRequest>,
    polymer_api_url: String,
}

impl ProofFetcher {
    pub fn new(
        event_rx: mpsc::Receiver<RelayEvent>,
        delivery_tx: mpsc::Sender<DeliveryRequest>,
        polymer_api_url: String,
    ) -> Self {
        Self {
            event_rx,
            delivery_tx,
            polymer_api_url,
        }
    }

    #[instrument(skip(self), name = "proof_fetcher_start")]
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting proof fetcher");

        while let Some(event) = self.event_rx.recv().await {
            let tx_hash = match event.tx_hash {
                Some(hash) => hash,
                None => {
                    error!("Event missing transaction hash");
                    continue;
                }
            };

            let proof_request = ProofRequest {
                event: event.clone(),
                tx_hash,
                destination_chain_id: event.destination_chain.chain_id,
                dest_contract_address: event.destination_chain.dest_dapp_address.clone(),
            };

            // Process proof request in a separate task
            let delivery_tx = self.delivery_tx.clone();
            let polymer_api_url = self.polymer_api_url.clone();

            tokio::spawn(async move {
                match Self::fetch_proof(proof_request.clone(), polymer_api_url).await {
                    Ok(proof) => {
                        let delivery_request = DeliveryRequest {
                            event,
                            proof,
                            destination_chain_id: proof_request.destination_chain_id,
                            destination_contract_address: proof_request.dest_contract_address,
                        };

                        if let Err(e) = delivery_tx.send(delivery_request).await {
                            error!(error = %e, "Failed to send delivery request");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to fetch proof");
                    }
                }
            });
        }

        Ok(())
    }

    #[instrument(skip(_polymer_api_url), fields(
        source_chain_id = ?request.event.source_chain.chain_id,
        dest_chain_id = ?request.event.destination_chain.chain_id,
        tx_hash = ?request.tx_hash
    ))]
    async fn fetch_proof(request: ProofRequest, _polymer_api_url: String) -> Result<Bytes> {
        info!("Fetching proof from Polymer API");

        // In a real implementation, we would make an HTTP request to the Polymer API
        // For now, we'll simulate a delay and return a dummy proof
        time::sleep(Duration::from_secs(2)).await;

        info!("Proof fetched successfully");

        // Return a dummy proof
        Ok(Bytes::from(vec![0u8; 32]))
    }
}
