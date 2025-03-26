import { ethers } from 'ethers';
import { DeliveryRequest } from '../types';
import { logger } from '../utils/logger';

const EXECUTOR_ABI = [
  'function executeWithProof(bytes calldata proof) external returns (bool success, bytes memory result)'
];

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


      // Get the destination contract address
      const destAddress = ethers.utils.getAddress(request.destinationContractAddress);

      // Create contract instance for the CrossChainExecutor
      const executorContract = new ethers.Contract(destAddress, EXECUTOR_ABI, wallet);

      // Get the proof
      const proof = request.proof;

      logger.info('Calling executeWithProof on destination contract', {
        destAddress: destAddress,
        proofLength: proof.length
      });

      // Call the executeWithProof function with the proof
      const tx = await executorContract.executeWithProof(proof, {
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
      const errorMessage = error instanceof Error ? error.message : String(error);
      if (error instanceof Error &&
          !errorMessage.includes('nonce too low') &&
          !errorMessage.includes('already known') &&
          !errorMessage.includes('RLP') &&
          !errorMessage.includes('INVALID_ARGUMENT')) {
        this.deliveryQueue.unshift(request);
      } else if (errorMessage.includes('RLP') || errorMessage.includes('INVALID_ARGUMENT')) {
         logger.error("RLP or INVALID_ARGUMENT error detected. Transaction data might be malformed. Not retrying automatically.");
      }
    } finally {
      // Process next event
      setTimeout(() => this.processQueue(), 0);
    }
  }
}
