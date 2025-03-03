mod client;

use self::client::ProofApiClient;
use crate::types::{DeliveryRequest, ProofRequest, RelayEvent};
use anyhow::Result;
use ethers::core::types::Bytes;
use tokio::{sync::mpsc};
use tracing::{error, info, instrument};

pub struct ProofFetcher {
    event_rx: mpsc::Receiver<RelayEvent>,
    delivery_tx: mpsc::Sender<DeliveryRequest>,
    polymer_api_url: String,
    api_token: String,
}

impl ProofFetcher {
    pub fn new(
        event_rx: mpsc::Receiver<RelayEvent>,
        delivery_tx: mpsc::Sender<DeliveryRequest>,
        polymer_api_url: String,
        api_token: String,
    ) -> Self {
        Self {
            event_rx,
            delivery_tx,
            polymer_api_url,
            api_token,
        }
    }

    #[instrument(skip(self), name = "proof_fetcher_start")]
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting proof fetcher");

        while let Some(event) = self.event_rx.recv().await {
            let tx_hash = match event.meta.tx_hash {
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
                dest_contract_address: event.dest_dapp_address.clone(),
            };

            // Process proof request in a separate task
            let delivery_tx = self.delivery_tx.clone();
            let polymer_api_url = self.polymer_api_url.clone();
            let api_token = self.api_token.clone();

            tokio::spawn(async move {
                match Self::fetch_proof(proof_request.clone(), polymer_api_url, api_token).await {
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

    #[instrument(skip(polymer_api_url, api_token), fields(
        source_chain_id = ?request.event.source_chain.chain_id,
        dest_chain_id = ?request.event.destination_chain.chain_id,
        tx_hash = ?request.tx_hash
    ))]
    async fn fetch_proof(
        request: ProofRequest, 
        polymer_api_url: String, 
        api_token: String
    ) -> Result<Bytes> {
        info!("Fetching proof from Polymer API");

        // Create the proof API client
        let client = ProofApiClient::new(api_token, polymer_api_url);
        
        // Request the proof from the Polymer API
        let proof = client.fetch_proof(
            request.event.source_chain.chain_id,
            request.event.meta.block_number,
            request.event.meta.tx_index,
            request.event.meta.log_index,
        ).await?;
        
        info!("Proof fetched successfully");
        
        Ok(proof)
    }
}
