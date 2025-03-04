// src/config/index.ts

import dotenv from 'dotenv';
import { ChainConfig, EnvVars, RelayerConfig, RelayPair } from './types';

// Load environment variables
dotenv.config();

/**
 * Get environment variables with validation
 */
export function getEnvVars(): EnvVars {
  const env = process.env as unknown as EnvVars;
  
  if (!env.PRIVATE_KEY) {
    throw new Error('PRIVATE_KEY environment variable is required');
  }
  
  return {
    PRIVATE_KEY: env.PRIVATE_KEY,
    POLLING_INTERVAL_MS: env.POLLING_INTERVAL_MS,
    POLYMER_API_ENDPOINT: env.POLYMER_API_ENDPOINT,
    POLYMER_API_TOKEN: env.POLYMER_API_TOKEN,
    LOG_LEVEL: env.LOG_LEVEL,
  };
}

/**
 * Default chain configurations
 */
export const defaultChainConfigs: Record<number, ChainConfig> = {
  11155420: {
    name: 'Optimism Sepolia',
    chainId: 11155420,
    rpcUrl: 'https://optimism-sepolia.example.com',
  },
  84532: {
    name: 'Base Sepolia',
    chainId: 84532,
    rpcUrl: 'https://base-sepolia.example.com',
  },
};

/**
 * Default relay pairs
 */
export const defaultRelayPairs: RelayPair[] = [
  {
    sourceChainId: 11155420,
    sourceResolverAddress: '0x1234567890123456789012345678901234567890',
    destChainId: 84532,
    destDappAddress: '0x0987654321098765432109876543210987654321',
  },
  {
    sourceChainId: 84532,
    sourceResolverAddress: '0x2345678901234567890123456789012345678901',
    destChainId: 11155420,
    destDappAddress: '0x9876543210987654321098765432109876543210',
  },
];

/**
 * Load relayer configuration
 */
export function loadConfig(): RelayerConfig {
  const env = getEnvVars();
  
  return {
    pollingIntervalMs: env.POLLING_INTERVAL_MS ? parseInt(env.POLLING_INTERVAL_MS, 10) : 10000,
    chains: defaultChainConfigs,
    relayPairs: defaultRelayPairs,
    polymerApi: {
      endpoint: env.POLYMER_API_ENDPOINT || 'https://api.polymer.zone/v1/proofs',
      token: env.POLYMER_API_TOKEN || 'your-api-token', // TODO: Require this in production
    },
  };
}

/**
 * Validate configuration
 */
export function validateConfig(config: RelayerConfig): void {
  // Ensure all chains referenced in relay pairs exist in config
  for (const pair of config.relayPairs) {
    if (!config.chains[pair.sourceChainId]) {
      throw new Error(`Source chain ${pair.sourceChainId} not found in chain configurations`);
    }
    
    if (!config.chains[pair.destChainId]) {
      throw new Error(`Destination chain ${pair.destChainId} not found in chain configurations`);
    }
  }
}

/**
 * Get relayer configuration
 */
export function getConfig(): RelayerConfig {
  const config = loadConfig();
  validateConfig(config);
  return config;
}

// Export types
export * from './types';
