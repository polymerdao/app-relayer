import { ethers } from 'ethers';
import { DeliveryRequest } from '../types';
import { logger } from '../utils/logger';

export class EventDeliverer {
  private isProcessing: boolean = false;
  private deliveryQueue: DeliveryRequest[] = [];

  constructor(private privateKey: string) {}

  /**
   * Deliver a cross-chain event to the destination chain
   */
  public async deliverEvent(request: DeliveryRequest): Promise<void> {
    this.deliveryQueue.push(request);
    logger.info(
      `Event added to delivery queue: destChain=${request.destinationChainId}, nonce=${request.event.nonce}`
    );

    if (!this.isProcessing) {
      this.processQueue();
    }
  }

  /**
   * Process the queue of delivery requests
   */
  private async processQueue(): Promise<void> {
    if (this.deliveryQueue.length === 0) {
      this.isProcessing = false;
      return;
    }

    this.isProcessing = true;
    const request = this.deliveryQueue.shift();

    if (!request) {
      this.isProcessing = false;
      return;
    }

    try {
      logger.info(
        `Delivering event to destination chain: destChain=${request.destinationChainId}, nonce=${request.event.nonce}`
      );

      // Connect to provider
      const provider = new ethers.providers.JsonRpcProvider(request.event.destinationChain.rpcUrl);
      
      // Create wallet
      const wallet = new ethers.Wallet(this.privateKey, provider);
      
      // Log function selector for debugging
      const functionSelector = request.event.execPayload.slice(0, 10); // "0x" + 8 chars
      logger.info(`Using function selector: ${functionSelector}`);

      // Create transaction data by concatenating execPayload and proof
      const proof = request.proof;
      const execPayload = request.event.execPayload;
      let txData: string;

      // Check if proof is already prefixed with 0x
      if (proof.startsWith('0x')) {
        txData = execPayload + proof.slice(2);
      } else {
        txData = execPayload + proof;
      }

      // Create contract instance with the destination address
      const destAddress = ethers.utils.getAddress(request.destinationContractAddress);
      
      // Send the transaction
      logger.info('Submitting transaction to destination chain');
      const tx = await wallet.sendTransaction({
        to: destAddress,
        data: txData,
        gasLimit: 500000, // Set an appropriate gas limit
      });

      logger.info(`Proof submission transaction sent: ${tx.hash}`);

      // Wait for transaction to be mined
      const receipt = await tx.wait();
      
      logger.info(
        `Proof submission confirmed: block=${receipt.blockNumber}, hash=${receipt.transactionHash}`
      );
    } catch (error) {
      logger.error(
        `Error delivering event: ${error instanceof Error ? error.message : String(error)}`,
        {
          destChainId: request.destinationChainId,
          destAddress: request.destinationContractAddress,
        }
      );

      // Re-queue the request if it was a temporary failure
      // In a production system, you might want to implement a backoff strategy
      if (error instanceof Error && 
          !error.message.includes('nonce too low') && 
          !error.message.includes('already known')) {
        this.deliveryQueue.unshift(request);
      }
    } finally {
      // Process next event
      setTimeout(() => this.processQueue(), 0);
    }
  }
}
