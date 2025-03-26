#!/usr/bin/env node

import dotenv from 'dotenv';
import { getConfig } from './config';
import { RelayerApp } from './app';
import { logger } from './utils/logger';

// Load environment variables
dotenv.config();

// Main entry point
async function main() {
  try {
    logger.info('Starting Cross-Chain Relayer');

    // Get private key from environment
    const privateKey = process.env.PRIVATE_KEY;
    if (!privateKey) {
      throw new Error('PRIVATE_KEY environment variable is required');
    }

    // Load and validate configuration
    const config = getConfig();
    logger.info('Configuration loaded', {
      chains: Object.keys(config.chains).length,
      relayPairs: config.relayPairs.length,
      pollingIntervalMs: config.pollingIntervalMs
    });

    // Create and start the relayer application
    const app = new RelayerApp(config, privateKey);
    await app.start();

    // Handle shutdown gracefully
    setupShutdownHandlers(app);

    logger.info('Relayer is running');
  } catch (error) {
    logger.error(`Failed to start relayer: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

// Set up handlers for graceful shutdown
function setupShutdownHandlers(app: RelayerApp) {
  const shutdown = () => {
    logger.info('Shutting down relayer...');
    app.stop();
    logger.info('Relayer stopped');
    process.exit(0);
  };

  // Handle termination signals
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
  process.on('uncaughtException', (error: Error) => {
    logger.error(`Uncaught exception: ${error.message}`, { stack: error.stack });
    shutdown();
  });
}

// Start the application
main().catch((error) => {
  logger.error(`Fatal error: ${error instanceof Error ? error.message : String(error)}`, {
    stack: error instanceof Error ? error.stack : undefined
  });
  process.exit(1);
});
