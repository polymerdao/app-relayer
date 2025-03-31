// src/config/index.ts

import dotenv from 'dotenv';
import fs from 'fs';
import path from 'path';
import yaml from 'js-yaml';
import { ChainConfig, EnvVars, RelayerConfig, RelayPair } from './types';
import { logger } from '../utils/logger';

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
    CONFIG_PATH: env.CONFIG_PATH,
  };
}

/**
 * Load configuration from YAML file
 */
export function loadConfigFromFile(filePath: string): RelayerConfig {
  try {
    const fileContents = fs.readFileSync(filePath, 'utf8');
    const config = yaml.load(fileContents) as RelayerConfig;
    
    // Convert chain configurations
    if (config.chains) {
      const chainsMap: Record<number, ChainConfig> = {};
      // If chains is an array in YAML, convert to map
      if (Array.isArray(config.chains)) {
        for (const chain of config.chains) {
          chainsMap[chain.chainId] = chain;
        }
        config.chains = chainsMap;
      }
    }
    
    return config;
  } catch (error) {
    throw new Error(`Failed to load config file: ${error instanceof Error ? error.message : String(error)}`);
  }
}

/**
 * Load relayer configuration
 */
export function loadConfig(): RelayerConfig {
  const env = getEnvVars();
  
  // First try to load from env var if specified
  let configPath: string;
  if (env.CONFIG_PATH) {
    configPath = env.CONFIG_PATH;
    logger.info(`Loading configuration from specified path: ${configPath}`);
  } else {
    configPath = path.join(process.cwd(), 'config.yaml');
    logger.info(`Loading configuration from default path: ${configPath}`);
  }
  
  if (!fs.existsSync(configPath)) {
    throw new Error(`Config file not found at ${configPath}`);
  }

  logger.info(`Loading configuration from ${configPath}`);
  const fileConfig = loadConfigFromFile(configPath);
  
  // Override with environment variables if provided
  if (env.POLLING_INTERVAL_MS) {
    fileConfig.pollingIntervalMs = parseInt(env.POLLING_INTERVAL_MS, 10);
  }
  
  if (env.POLYMER_API_ENDPOINT) {
    fileConfig.polymerApi.endpoint = env.POLYMER_API_ENDPOINT;
  }
  
  if (env.POLYMER_API_TOKEN) {
    fileConfig.polymerApi.token = env.POLYMER_API_TOKEN;
  }
  
  return fileConfig;
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
