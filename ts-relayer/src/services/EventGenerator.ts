import { ethers } from 'ethers';
import { ChainConfig, RelayPair } from '../config/types';
import { logger } from '../utils/logger';
import { RelayEvent, EventMeta } from '../types';

// ABI fragments for the cross-chain resolver interface
const RESOLVER_ABI = [
  'function crossChainChecker(uint32 destinationChainId) external view returns (bool canExec, bytes memory execPayload, uint256 nonce)',
  'function requestRemoteExecution(uint32 destinationChainId) external',
  'event CrossChainExecRequested(uint32 indexed destinationChainId, bytes execPayload, uint256 indexed nonce)'
];

export class EventGenerator {
  private chains: Record<number, ChainConfig>;
  private relayPairs: RelayPair[];
  private privateKey: string;
  private pollingIntervalMs: number;
  private eventCallback: (event: RelayEvent) => Promise<void>;
  private running: boolean = false;

  constructor(
    chains: Record<number, ChainConfig>,
    relayPairs: RelayPair[],
    privateKey: string,
    pollingIntervalMs: number,
    eventCallback: (event: RelayEvent) => Promise<void>
  ) {
    this.chains = chains;
    this.relayPairs = relayPairs;
    this.privateKey = privateKey;
    this.pollingIntervalMs = pollingIntervalMs;
    this.eventCallback = eventCallback;
  }

  public async start(): Promise<void> {
    logger.info('Starting event generator');
    this.running = true;
    
    while (this.running) {
      try {
        await this.checkAllChains();
      } catch (error) {
        logger.error({ error }, 'Error checking chains');
      }
      
      // Wait for the polling interval
      await new Promise(resolve => setTimeout(resolve, this.pollingIntervalMs));
    }
  }

  public stop(): void {
    logger.info('Stopping event generator');
    this.running = false;
  }

  private async checkAllChains(): Promise<void> {
    for (const relayPair of this.relayPairs) {
      const sourceChain = this.chains[relayPair.sourceChainId];
      const destChain = this.chains[relayPair.destChainId];

      if (!sourceChain) {
        logger.error(`Source chain ${relayPair.sourceChainId} not found in config`);
        continue;
      }

      if (!destChain) {
        logger.error(`Destination chain ${relayPair.destChainId} not found in config`);
        continue;
      }

      try {
        await this.checkCrossChainEvents(sourceChain, destChain, relayPair);
      } catch (error) {
        logger.error({
          sourceChain: sourceChain.name,
          destChain: destChain.name,
          error
        }, 'Error checking cross-chain events');
      }
    }
  }

  private async checkCrossChainEvents(
    sourceChain: ChainConfig,
    destChain: ChainConfig,
    relayPair: RelayPair
  ): Promise<void> {
    logger.info({
      sourceChain: sourceChain.name,
      destChain: destChain.name
    }, 'Checking cross-chain events');

    // Connect to provider
    const provider = new ethers.providers.JsonRpcProvider(sourceChain.rpcUrl);
    
    // Create wallet
    const wallet = new ethers.Wallet(this.privateKey, provider);
    
    // Create resolver contract interface
    const resolverAddress = ethers.utils.getAddress(relayPair.sourceResolverAddress);
    const resolverContract = new ethers.Contract(resolverAddress, RESOLVER_ABI, wallet);

    logger.debug('Calling crossChainChecker() on resolver');

    // Call the crossChainChecker function
    const destChainIdU32 = destChain.chainId;
    const [canExec, execPayload, nonce] = await resolverContract.crossChainChecker(destChainIdU32);

    if (canExec) {
      logger.info({
        nonce: nonce.toString(),
        sourceChain: sourceChain.name,
        destChain: destChain.name
      }, '✅ Cross-chain execution needed');

      // Process the cross-chain event
      const txHash = await this.requestRemoteExecution(sourceChain, destChain, relayPair);

      // Extract event details and create the RelayEvent
      const event = await this.extractEventDetails(
        txHash,
        sourceChain,
        destChain,
        execPayload,
        nonce.toString(),
        relayPair
      );

      // Send the event to the proof fetcher via callback
      await this.eventCallback(event);
    } else {
      logger.debug('⏳ No cross-chain execution needed');
    }
  }

  private async requestRemoteExecution(
    sourceChain: ChainConfig,
    destChain: ChainConfig,
    relayPair: RelayPair
  ): Promise<string> {
    logger.info('Requesting remote execution');

    // Connect to provider
    const provider = new ethers.providers.JsonRpcProvider(sourceChain.rpcUrl);
    
    // Create wallet
    const wallet = new ethers.Wallet(this.privateKey, provider);
    
    // Create resolver contract interface
    const resolverAddress = ethers.utils.getAddress(relayPair.sourceResolverAddress);
    const resolverContract = new ethers.Contract(resolverAddress, RESOLVER_ABI, wallet);

    // Call requestRemoteExecution
    logger.info('Calling requestRemoteExecution on resolver');
    const tx = await resolverContract.requestRemoteExecution(destChain.chainId);

    logger.info({ txHash: tx.hash }, 'Transaction sent');

    // Wait for transaction to be mined
    const receipt = await tx.wait();
    logger.info({ 
      txHash: receipt.transactionHash,
      blockNumber: receipt.blockNumber
    }, 'Transaction confirmed');

    return receipt.transactionHash;
  }

  private async extractEventDetails(
    txHash: string,
    sourceChain: ChainConfig,
    destChain: ChainConfig,
    execPayload: string,
    nonce: string,
    relayPair: RelayPair
  ): Promise<RelayEvent> {
    // Get the transaction receipt to extract event details
    const provider = new ethers.providers.JsonRpcProvider(sourceChain.rpcUrl);
    const txReceipt = await provider.getTransactionReceipt(txHash);

    if (!txReceipt) {
      throw new Error('Transaction receipt not found');
    }

    // Create ABI interface for parsing logs
    const iface = new ethers.utils.Interface(RESOLVER_ABI);
    
    // Find the CrossChainExecRequested event in the logs
    const resolverAddress = ethers.utils.getAddress(relayPair.sourceResolverAddress);
    const eventSignature = 'CrossChainExecRequested(uint32,bytes,uint256)';
    const eventTopic = ethers.utils.id(eventSignature);
    
    const crossChainEvent = txReceipt.logs.find(log => 
      log.address.toLowerCase() === resolverAddress.toLowerCase() && 
      log.topics[0] === eventTopic
    );

    if (!crossChainEvent) {
      throw new Error('CrossChainExecRequested event not found in transaction');
    }

    // Create a relay event with actual transaction details
    const event: RelayEvent = {
      sourceChain: sourceChain,
      sourceResolverAddress: relayPair.sourceResolverAddress,
      destinationChain: destChain,
      destDappAddress: relayPair.destDappAddress,
      execPayload: execPayload,
      nonce: nonce,
      meta: {
        txHash: txHash,
        blockNumber: txReceipt.blockNumber,
        txIndex: txReceipt.transactionIndex,
        logIndex: crossChainEvent.logIndex,
      },
    };

    return event;
  }
}
