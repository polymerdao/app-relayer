use crate::config::RelayPair;
use crate::types::{ChainConfig, EventMeta, RelayEvent};
use anyhow::anyhow;
use anyhow::{Context, Result};
use ethers::{
    abi::{self},
    core::types::{Address, Bytes, H256, U256},
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    utils::keccak256,
};
use std::collections::HashMap;
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::{sync::mpsc, time};
use tracing::{debug, error, info, instrument};

pub struct EventGenerator {
    chains: HashMap<u64, ChainConfig>,
    relay_pairs: Vec<RelayPair>,
    private_key: String,
    polling_interval: Duration,
    event_tx: mpsc::Sender<RelayEvent>,
}

impl EventGenerator {
    pub fn new(
        chains: HashMap<u64, ChainConfig>,
        relay_pairs: Vec<RelayPair>,
        private_key: String,
        polling_interval: Duration,
        event_tx: mpsc::Sender<RelayEvent>,
    ) -> Self {
        Self {
            chains,
            private_key,
            polling_interval,
            event_tx,
            relay_pairs,
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
        for relay_pair in &self.relay_pairs {
            let source_chain = self
                .chains
                .get(&relay_pair.source_chain_id)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Source chain {} not found in config",
                        relay_pair.source_chain_id
                    )
                })?;

            let dest_chain = self.chains.get(&relay_pair.dest_chain_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Destination chain {} not found in config",
                    relay_pair.dest_chain_id
                )
            })?;

            match self
                .check_cross_chain_events(source_chain, dest_chain, relay_pair)
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
        Ok(())
    }

    #[instrument(skip(self), fields(source_chain = %source_chain.name, dest_chain = %dest_chain.name))]
    async fn check_cross_chain_events(
        &self,
        source_chain: &ChainConfig,
        dest_chain: &ChainConfig,
        relay_pair: &RelayPair,
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
        let resolver_address = Address::from_str(&relay_pair.source_resolver_address)
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
                source_chain = source_chain.name,
                dest_chain = dest_chain.name,
                "✅ Cross-chain execution needed"
            );

            // Process the cross-chain event
            let tx_hash = self
                .request_remote_execution(&source_chain, relay_pair)
                .await?;

            // Extract event details and create the RelayEvent
            let event = self
                .extract_event_details(
                    tx_hash,
                    source_chain,
                    dest_chain,
                    exec_payload,
                    nonce.as_u64(),
                    relay_pair,
                )
                .await?;

            // Send the event to the proof fetcher
            if let Err(e) = self.event_tx.send(event).await {
                error!(error = %e, "Failed to send event to proof fetcher");
            }
        } else {
            debug!("⏳ No cross-chain execution needed");
        }

        Ok(())
    }

    #[instrument(skip(self), fields(source_chain = %source_chain.name, dest_chain = %destination_chain.name))]
    async fn extract_event_details(
        &self,
        tx_hash: H256,
        source_chain: &ChainConfig,
        destination_chain: &ChainConfig,
        exec_payload: Bytes,
        nonce: u64,
        relay_pair: &RelayPair,
    ) -> Result<RelayEvent> {
        // Get the transaction receipt to extract event details
        let provider = Provider::<Http>::try_from(&source_chain.rpc_url).context(format!(
            "Failed to create provider for {}",
            source_chain.name
        ))?;
        let tx_receipt = provider
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        // Find the CrossChainExecRequested event in the logs
        let cross_chain_event = tx_receipt
            .logs
            .iter()
            .find(|log| {
                // Check if this log is from our source resolver address
                let from_resolver = log.address
                    == Address::from_str(&relay_pair.source_resolver_address).unwrap_or_default();

                // Check if the log has the CrossChainExecRequested event signature
                // Event: CrossChainExecRequested(uint32 indexed destinationChainId, bytes execPayload, uint256 indexed nonce)
                // Keccak256 hash of the event signature
                let event_signature = "CrossChainExecRequested(uint32,bytes,uint256)";
                let event_signature_hash = keccak256(event_signature.as_bytes());

                from_resolver
                    && log
                        .topics
                        .get(0)
                        .map_or(false, |t| t.as_bytes() == &event_signature_hash[..])
            })
            .ok_or_else(|| {
                anyhow::anyhow!("CrossChainExecRequested event not found in transaction")
            })?;

        // Create a relay event with actual transaction details
        let event = RelayEvent {
            source_chain: source_chain.clone(),
            source_resolver_address: relay_pair.source_resolver_address.clone(),
            destination_chain: destination_chain.clone(),
            dest_dapp_address: relay_pair.dest_dapp_address.clone(),
            exec_payload,
            nonce,
            meta: EventMeta {
                tx_hash: Some(tx_hash),
                block_number: tx_receipt
                    .block_number
                    .map(|n| n.as_u64())
                    .ok_or(anyhow!("block_number not found from receipt"))?,
                tx_index: tx_receipt.transaction_index.as_u32(),
                log_index: cross_chain_event
                    .log_index
                    .map(|n| n.as_u32())
                    .ok_or(anyhow!(
                        "log_index not found from CrossChainExecRequested event"
                    ))?,
            },
        };

        Ok(event)
    }

    async fn request_remote_execution(
        &self,
        source_chain: &ChainConfig,
        relay_pair: &RelayPair,
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
        let resolver_address = Address::from_str(&relay_pair.source_resolver_address)
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
            .method::<_, ()>("requestRemoteExecution", relay_pair.dest_chain_id)?;
        let tx = tx_req.send().await?;

        let tx_hash = tx.tx_hash();
        info!(?tx_hash, "Transaction sent");

        // Wait for transaction to be mined
        let receipt = tx
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        info!(?receipt, "Transaction confirmed");

        Ok(tx_hash)
    }
}
