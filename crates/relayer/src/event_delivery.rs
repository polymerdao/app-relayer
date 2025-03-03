use crate::types::DeliveryRequest;
use anyhow::{Context, Result};
use ethers::{
    abi::{self},
    core::types::Address,
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
};
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

pub struct EventDeliverer {
    private_key: String,
    delivery_rx: mpsc::Receiver<DeliveryRequest>,
}

impl EventDeliverer {
    pub fn new(private_key: String, delivery_rx: mpsc::Receiver<DeliveryRequest>) -> Self {
        Self {
            private_key,
            delivery_rx,
        }
    }

    #[instrument(skip(self), name = "event_deliverer_start")]
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting event deliverer");

        while let Some(delivery) = self.delivery_rx.recv().await {
            // Process delivery in a separate task to allow concurrent deliveries
            let private_key = self.private_key.clone();

            tokio::spawn(async move {
                match Self::deliver_event(delivery, private_key).await {
                    Ok(_) => {
                        info!("Event delivered successfully");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to deliver event");
                    }
                }
            });
        }

        Ok(())
    }

    #[instrument(skip(private_key), fields(
        source_chain = %delivery.event.source_chain.name,
        dest_chain = %delivery.event.destination_chain.name,
        nonce = delivery.event.nonce
    ))]
    async fn deliver_event(delivery: DeliveryRequest, private_key: String) -> Result<()> {
        let dest_chain = delivery.event.destination_chain;

        info!("Delivering event to destination chain");

        // Connect to provider
        let provider = Provider::<Http>::try_from(&dest_chain.rpc_url)
            .context(format!("Failed to create provider for {}", dest_chain.name))?;
        let client = Arc::new(provider);

        // Create wallet
        let wallet = LocalWallet::from_str(&private_key)
            .context("Failed to create wallet")?
            .with_chain_id(dest_chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);

        // Create target contract interface
        let state_sync_address = Address::from_str(&dest_chain.state_sync_address)
            .context("Invalid state sync address")?;

        // Create ABI for the state sync contract
        let state_sync_abi =
            abi::parse_abi(&["function setValueFromSource(bytes calldata proof) external"])?;
        let state_sync_contract =
            Contract::new(state_sync_address, state_sync_abi, Arc::new(client));

        // Submit proof to destination chain
        info!("Submitting proof to destination chain");
        let tx_req = state_sync_contract.method::<_, ()>("setValueFromSource", delivery.proof)?;
        let tx = tx_req.send().await?;

        let tx_hash = tx.tx_hash();
        info!("Proof submission transaction sent: {:?}", tx_hash);

        // Wait for transaction to be mined
        let receipt = tx
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        info!("Proof submission confirmed: {:?}", receipt);

        Ok(())
    }
}
