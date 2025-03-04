import axios from 'axios';
import { ethers } from 'ethers';
import { RelayEvent, DeliveryRequest } from '../types';
import { logger } from '../utils/logger';

interface PolymerApiConfig {
  endpoint: string;
  token: string;
}

interface RequestProofResponse {
  result: number;
}

interface QueryProofResult {
  status: string;
  proof: string;
}

interface QueryProofResponse {
  result: QueryProofResult;
}

export class ProofFetcher {
  private eventQueue: RelayEvent[] = [];
  private isProcessing = false;
  private polymerApi: PolymerApiConfig;

  constructor(
    private deliveryCallback: (request: DeliveryRequest) => Promise<void>,
    polymerApi: PolymerApiConfig
  ) {
    this.polymerApi = polymerApi;
  }

  /**
   * Add a new event to the proof fetcher queue
   */
  public async addEvent(event: RelayEvent): Promise<void> {
    this.eventQueue.push(event);
    logger.info(
      `Event added to proof fetcher queue: sourceChain=${event.sourceChain.chainId}, destChain=${event.destinationChain.chainId}, nonce=${event.nonce}`
    );

    if (!this.isProcessing) {
      this.processQueue();
    }
  }

  /**
   * Process the queue of events
   */
  private async processQueue(): Promise<void> {
    if (this.eventQueue.length === 0) {
      this.isProcessing = false;
      return;
    }

    this.isProcessing = true;
    const event = this.eventQueue.shift();

    if (!event) {
      this.isProcessing = false;
      return;
    }

    try {
      if (!event.meta.txHash) {
        throw new Error('Event missing transaction hash');
      }

      logger.info(
        `Fetching proof for event: sourceChain=${event.sourceChain.chainId}, destChain=${event.destinationChain.chainId}, nonce=${event.nonce}`
      );

      // Fetch proof from Polymer API
      const proof = await this.fetchProof(
        event.sourceChain.chainId,
        event.meta.blockNumber,
        event.meta.txIndex,
        event.meta.logIndex
      );

      // Create delivery request
      const deliveryRequest: DeliveryRequest = {
        destinationChainId: event.destinationChain.chainId,
        destinationContractAddress: event.destDappAddress,
        event,
        proof,
      };

      // Send to delivery service
      await this.deliveryCallback(deliveryRequest);
      logger.info(
        `Proof fetched and delivery requested: sourceChain=${event.sourceChain.chainId}, destChain=${event.destinationChain.chainId}, nonce=${event.nonce}`
      );
    } catch (error) {
      logger.error(
        `Error fetching proof: ${error instanceof Error ? error.message : String(error)}`,
        {
          sourceChainId: event.sourceChain.chainId,
          destChainId: event.destinationChain.chainId,
          txHash: event.meta.txHash,
        }
      );

      // Re-queue the event if it was a temporary failure
      // In a production system, you might want to implement a backoff strategy
      if (error instanceof Error && !error.message.includes('not found')) {
        this.eventQueue.unshift(event);
      }
    } finally {
      // Process next event
      setTimeout(() => this.processQueue(), 0);
    }
  }

  /**
   * Fetch proof from Polymer API
   */
  private async fetchProof(
    chainId: number,
    blockNumber: number,
    txIndex: number,
    logIndex: number
  ): Promise<string> {
    // Request proof generation
    const jobId = await this.requestProof(chainId, blockNumber, txIndex, logIndex);
    
    // Poll for proof completion
    let attempts = 0;
    const maxAttempts = 5;
    
    while (attempts < maxAttempts) {
      const result = await this.queryProof(jobId);
      
      if (result.status === 'ready' || result.status === 'complete') {
        return result.proof;
      }
      
      attempts++;
      await new Promise(resolve => setTimeout(resolve, 2000)); // 2 second delay
    }
    
    throw new Error('Timeout waiting for proof');
  }

  /**
   * Request proof generation from Polymer API
   */
  private async requestProof(
    chainId: number,
    blockNumber: number,
    txIndex: number,
    logIndex: number
  ): Promise<number> {
    try {
      const response = await axios.post(
        this.polymerApi.endpoint,
        {
          jsonrpc: '2.0',
          id: 1,
          method: 'log_requestProof',
          params: [chainId, blockNumber, txIndex, logIndex],
        },
        {
          headers: {
            Authorization: `Bearer ${this.polymerApi.token}`,
          },
        }
      );

      logger.debug('Raw proof response', {
        method: 'log_requestProof',
        response: JSON.stringify(response.data),
      });

      const data = response.data as RequestProofResponse;
      return data.result;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        logger.error('Polymer API error', {
          status: error.response?.status,
          data: error.response?.data,
        });
      }
      throw new Error(`Failed to request proof: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Query proof status from Polymer API
   */
  private async queryProof(jobId: number): Promise<QueryProofResult> {
    try {
      const response = await axios.post(
        this.polymerApi.endpoint,
        {
          jsonrpc: '2.0',
          id: 1,
          method: 'log_queryProof',
          params: [jobId],
        }
      );

      logger.debug('Raw query response', {
        method: 'log_queryProof',
        response: JSON.stringify(response.data),
      });

      const data = response.data as QueryProofResponse;
      return data.result;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        logger.error('Polymer API error', {
          status: error.response?.status,
          data: error.response?.data,
        });
      }
      throw new Error(`Failed to query proof: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}
