use crate::types::{ChainConfig, RelayEvent};
use anyhow::{Context, Result};
use ethers::{
    abi::{self},
    core::types::{Address, Bytes, H256, U256},
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::{sync::mpsc, time};
use tracing::{debug, error, info, instrument};

pub struct EventGenerator {
    chains: Vec<Arc<ChainConfig>>,
    private_key: String,
    polling_interval: Duration,
    event_tx: mpsc::Sender<RelayEvent>,
}

impl EventGenerator {
    pub fn new(
        chains: Vec<Arc<ChainConfig>>,
        private_key: String,
        polling_interval: Duration,
        event_tx: mpsc::Sender<RelayEvent>,
    ) -> Self {
        Self {
            chains,
            private_key,
            polling_interval,
            event_tx,
        }
    }

    #[instrument(skip(self), name = "event_generator_start")]
    pub async fn start(&self) -> Result<()> {
        info!("Starting event generator");

        let mut interval_timer = time::interval(self.polling_interval);

        loop {
            interval_timer.tick().await;
            if let Err(e) = self.check_all_chains().await {
                error!(error = %e, "Error checking chains");
            }
        }
    }

    #[instrument(skip(self))]
    async fn check_all_chains(&self) -> Result<()> {
        for source_chain in &self.chains {
            // Check for cross-chain events for each destination chain
            for dest_chain in &self.chains {
                if source_chain.chain_id != dest_chain.chain_id {
                    match self
                        .check_cross_chain_events(source_chain.clone(), dest_chain.clone())
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => error!(
                            source_chain = %source_chain.name,
                            dest_chain = %dest_chain.name,
                            error = %e,
                            "Error checking cross-chain events"
                        ),
                    }
                }
            }
        }
        Ok(())
    }

    #[instrument(skip(self), fields(source_chain = %source_chain.name, dest_chain = %dest_chain.name))]
    async fn check_cross_chain_events(
        &self,
        source_chain: Arc<ChainConfig>,
        dest_chain: Arc<ChainConfig>,
    ) -> Result<()> {
        info!("Checking cross-chain events");

        // Connect to provider
        let provider = Provider::<Http>::try_from(&source_chain.rpc_url).context(format!(
            "Failed to create provider for {}",
            source_chain.name
        ))?;
        let client = Arc::new(provider);

        // Create wallet
        let wallet = LocalWallet::from_str(&self.private_key)
            .context("Failed to create wallet")?
            .with_chain_id(source_chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);

        // Create resolver contract interface
        let resolver_address = Address::from_str(&source_chain.resolver_address)
            .context("Invalid resolver address")?;

        // Create ABI for the cross-chain resolver interface
        let resolver_abi = abi::parse_abi(&[
            "function crossChainChecker(uint32 destinationChainId) external view returns (bool canExec, bytes memory execPayload, uint256 nonce)"
        ])?;
        let resolver_contract =
            Contract::new(resolver_address, resolver_abi, Arc::new(client.clone()));

        debug!("Calling crossChainChecker() on resolver");

        // Call the crossChainChecker function
        let dest_chain_id_u32 = dest_chain.chain_id as u32;
        let result: (bool, Bytes, U256) = resolver_contract
            .method("crossChainChecker", dest_chain_id_u32)?
            .call()
            .await?;

        let (can_exec, exec_payload, nonce) = result;

        if can_exec {
            info!(
                nonce = nonce.as_u64(),
                "✅ Cross-chain execution needed from {} to {}", source_chain.name, dest_chain.name
            );

            // Process the cross-chain event
            let tx_hash = self
                .request_remote_execution(
                    source_chain.clone(),
                    dest_chain.clone(),
                    dest_chain_id_u32,
                )
                .await?;

            // Create a relay event
            let event = RelayEvent {
                source_chain,
                destination_chain: dest_chain,
                exec_payload,
                tx_hash: Some(tx_hash),
                nonce: nonce.as_u64(),
            };

            // Send the event to the proof fetcher
            if let Err(e) = self.event_tx.send(event).await {
                error!(error = %e, "Failed to send event to proof fetcher");
            }
        } else {
            debug!("⏳ No cross-chain execution needed");
        }

        Ok(())
    }

    #[instrument(skip(self), fields(source_chain = %source_chain.name, dest_chain = %dest_chain.name))]
    async fn request_remote_execution(
        &self,
        source_chain: Arc<ChainConfig>,
        dest_chain: Arc<ChainConfig>,
        dest_chain_id: u32,
    ) -> Result<H256> {
        info!("Requesting remote execution");

        // Connect to provider
        let provider = Provider::<Http>::try_from(&source_chain.rpc_url).context(format!(
            "Failed to create provider for {}",
            source_chain.name
        ))?;
        let client = Arc::new(provider);

        // Create wallet
        let wallet = LocalWallet::from_str(&self.private_key)
            .context("Failed to create wallet")?
            .with_chain_id(source_chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);

        // Create resolver contract interface
        let resolver_address = Address::from_str(&source_chain.resolver_address)
            .context("Invalid resolver address")?;

        // Create ABI for the cross-chain resolver interface
        let resolver_abi = abi::parse_abi(&[
            "function requestRemoteExecution(uint32 destinationChainId) external",
        ])?;
        let resolver_contract =
            Contract::new(resolver_address, resolver_abi, Arc::new(client.clone()));

        // Call requestRemoteExecution
        info!("Calling requestRemoteExecution on resolver");
        let tx_req = resolver_contract
            .method::<_, ()>("requestRemoteExecution", dest_chain_id)?;
        let tx = tx_req.send().await?;

        let tx_hash = tx.tx_hash();
        info!(?tx_hash, "Transaction sent");

        // Wait for transaction to be mined
        let receipt = tx
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        info!("Transaction confirmed: {:?}", receipt);

        Ok(tx_hash)
    }
}
