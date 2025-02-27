use ethers::{
    abi::{self, ParamType, Token},
    core::types::{Address, Bytes, TransactionRequest, U256},
    prelude::*,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time;

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

// Main relayer struct
pub struct GenericRelayer {
    config: RelayerConfig,
    private_key: String,
}

impl GenericRelayer {
    pub fn new(config: RelayerConfig, private_key: String) -> Self {
        Self { config, private_key }
    }

    // Start the relayer
    pub async fn start(&self) -> Result<()> {
        println!("Starting generic relayer...");
        
        // Start polling loop
        let interval = Duration::from_millis(self.config.polling_interval_ms);
        let mut interval_timer = time::interval(interval);
        
        loop {
            interval_timer.tick().await;
            self.check_all_chains().await?;
        }
    }

    // Check all chains for pending operations
    async fn check_all_chains(&self) -> Result<()> {
        for chain in &self.config.chains {
            match self.check_chain(chain).await {
                Ok(_) => {},
                Err(e) => println!("Error checking chain {}: {:?}", chain.name, e),
            }
        }
        Ok(())
    }

    // Check a single chain for pending operations
    async fn check_chain(&self, chain: &ChainConfig) -> Result<()> {
        println!("Checking chain: {}", chain.name);
        
        // Connect to provider
        let provider = Provider::<Http>::try_from(&chain.rpc_url)?;
        let client = Arc::new(provider);
        
        // Create wallet
        let wallet = LocalWallet::from_str(&self.private_key)?
            .with_chain_id(chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);
        
        // Create resolver contract interface
        let resolver_address = Address::from_str(&chain.resolver_address)?;
        
        // Create minimal ABI for the resolver's checker function
        // This is the key part - we only need to know about the resolver interface
        let resolver_abi = abi::parse_abi(&[
            "function checker() external view returns (bool canExec, bytes memory execPayload)"
        ])?;
        let resolver_contract = Contract::new(resolver_address, resolver_abi, Arc::new(client.clone()));
        
        // Call the checker function
        println!("Calling checker() on resolver...");
        
        // Using low-level call to handle the tuple return type
        let result: (bool, Bytes) = resolver_contract
            .method("checker", ())?
            .call()
            .await?;
        
        let (can_exec, exec_payload) = result;
        
        if can_exec {
            println!("✅ Execution needed on chain {}", chain.name);
            self.process_operation(chain, exec_payload).await?;
        } else {
            println!("⏳ No execution needed on chain {}", chain.name);
        }
        
        Ok(())
    }

    // Process a cross-chain operation
    async fn process_operation(&self, source_chain: &ChainConfig, exec_payload: Bytes) -> Result<()> {
        println!("Processing operation on {}...", source_chain.name);
        
        // Create provider and client
        let provider = Provider::<Http>::try_from(&source_chain.rpc_url)?;
        let client = Arc::new(provider);
        
        // Create wallet
        let wallet = LocalWallet::from_str(&self.private_key)?
            .with_chain_id(source_chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);
        
        // This is the critical part - dynamically constructing a transaction from the exec_payload
        // We don't need to decode the payload because it's already encoded for the target function
        let tx = TransactionRequest::new()
            .to(source_chain.resolver_address.parse::<Address>()?)
            .data(exec_payload);
        
        // Send the transaction
        println!("Sending transaction to execute payload...");
        let tx_hash = client.send_transaction(tx, None).await?.tx_hash();
        println!("Transaction sent: {:?}", tx_hash);
        
        // Wait for transaction to be mined
        let receipt = client
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;
        
        println!("Transaction confirmed: {:?}", receipt);
        
        // For each destination chain, relay the message
        for dest_chain in &self.config.chains {
            if dest_chain.chain_id != source_chain.chain_id {
                self.relay_to_destination(source_chain, dest_chain, tx_hash)
                    .await?;
            }
        }
        
        Ok(())
    }

    // Relay a message to a destination chain
    async fn relay_to_destination(
        &self,
        source_chain: &ChainConfig,
        dest_chain: &ChainConfig,
        tx_hash: H256,
    ) -> Result<()> {
        println!(
            "Relaying from {} to {}...",
            source_chain.name, dest_chain.name
        );
        
        // Request proof from Polymer API (dummy implementation)
        let proof = self.request_proof(
            source_chain.chain_id,
            dest_chain.chain_id,
            tx_hash,
        ).await?;
        
        // Create provider and client for destination chain
        let provider = Provider::<Http>::try_from(&dest_chain.rpc_url)?;
        let client = Arc::new(provider);
        
        // Create wallet
        let wallet = LocalWallet::from_str(&self.private_key)?
            .with_chain_id(dest_chain.chain_id);
        let client = SignerMiddleware::new(client, wallet);
        
        // Create target contract interface with minimal ABI
        let dest_address = Address::from_str(&dest_chain.state_sync_address)?;
        let dest_abi = abi::parse_abi(&[
            "function setValueFromSource(bytes calldata proof) external"
        ])?;
        let dest_contract = Contract::new(dest_address, dest_abi, Arc::new(client));
        
        // Submit proof to destination chain
        println!("Submitting proof to {}...", dest_chain.name);
        let tx = dest_contract
            .method("setValueFromSource", proof)?
            .send()
            .await?
            .await?
            .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;
        
        println!("Proof confirmed on {}, tx: {:?}", dest_chain.name, tx);
        
        Ok(())
    }

    // Request proof from Polymer Prover API (dummy implementation)
    async fn request_proof(
        &self,
        source_chain_id: u64,
        dest_chain_id: u64,
        tx_hash: H256,
    ) -> Result<Bytes> {
        println!(
            "Requesting proof for tx {:?} from chain {} to {}",
            tx_hash, source_chain_id, dest_chain_id
        );
        
        // In a real implementation, this would call the Polymer Prover API
        // For now, return a dummy proof
        Ok(Bytes::from(vec![0u8; 32]))
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration (would be from a file in a real application)
    let config = RelayerConfig {
        polling_interval_ms: 60000,
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
    
    // Create and start relayer (private key would come from env or secure storage)
    let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let relayer = GenericRelayer::new(config, private_key.to_string());
    relayer.start().await?;
    
    Ok(())
}
