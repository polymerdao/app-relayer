import { RelayerConfig } from './config/types';
import { EventGenerator, ProofFetcher, EventDeliverer } from './services';
import { logger } from './utils/logger';
import { RelayEvent, DeliveryRequest } from './types';

/**
 * Main application class that coordinates all relayer components
 */
export class RelayerApp {
  private eventGenerator: EventGenerator;
  private proofFetcher: ProofFetcher;
  private eventDeliverer: EventDeliverer;
  private isRunning: boolean = false;

  /**
   * Creates a new RelayerApp instance
   * 
   * @param config The relayer configuration
   * @param privateKey Private key for transaction signing
   */
  constructor(config: RelayerConfig, privateKey: string) {
    logger.info('Initializing relayer application');

    // Create the event deliverer
    this.eventDeliverer = new EventDeliverer(privateKey);

    // Create the proof fetcher with a callback to the event deliverer
    this.proofFetcher = new ProofFetcher(
      (request: DeliveryRequest) => this.eventDeliverer.deliverEvent(request),
      {
        endpoint: config.polymerApi.endpoint,
        token: config.polymerApi.token
      }
    );

    // Create the event generator with a callback to the proof fetcher
    this.eventGenerator = new EventGenerator(
      config.chains,
      config.relayPairs,
      privateKey,
      config.pollingIntervalMs,
      (event: RelayEvent) => this.proofFetcher.addEvent(event)
    );
  }

  /**
   * Starts all relayer components
   */
  public async start(): Promise<void> {
    if (this.isRunning) {
      logger.warn('Relayer is already running');
      return;
    }

    logger.info('Starting all relayer components');
    this.isRunning = true;

    // Start the event generator
    // This will begin polling for cross-chain events
    await this.eventGenerator.start();
  }

  /**
   * Stops all relayer components
   */
  public stop(): void {
    if (!this.isRunning) {
      logger.warn('Relayer is not running');
      return;
    }

    logger.info('Stopping all relayer components');
    this.isRunning = false;

    // Stop the event generator
    this.eventGenerator.stop();
  }
}

/**
 * Helper function to create and start a relayer from a config
 */
export async function startRelayer(config: RelayerConfig, privateKey: string): Promise<RelayerApp> {
  const app = new RelayerApp(config, privateKey);
  await app.start();
  return app;
}
