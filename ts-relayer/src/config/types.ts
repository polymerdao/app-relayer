// src/config/types.ts

/**
 * Chain configuration
 */
export interface ChainConfig {
  /** Chain name */
  name: string;
  /** Chain ID */
  chainId: number;
  /** RPC URL for chain */
  rpcUrl: string;
}

/**
 * Source-destination pair for relaying
 */
export interface RelayPair {
  /** Source chain ID */
  sourceChainId: number;
  /** Source resolver contract address */
  sourceResolverAddress: string;
  /** Destination chain ID */
  destChainId: number;
  /** Destination dapp address */
  destDappAddress: string;
}

/**
 * Main configuration structure
 */
export interface RelayerConfig {
  /** Polling interval in milliseconds */
  pollingIntervalMs: number;
  /** Map of chain ID to chain config */
  chains: Record<number, ChainConfig>;
  /** Relay pairs to monitor */
  relayPairs: RelayPair[];
  /** Polymer API configuration */
  polymerApi: {
    /** API endpoint URL */
    endpoint: string;
    /** API token */
    token: string;
  };
}

/**
 * Environment variables structure
 */
export interface EnvVars {
  /** Private key for transaction signing */
  PRIVATE_KEY: string;
  /** Polling interval in milliseconds */
  POLLING_INTERVAL_MS?: string;
  /** Polymer API endpoint */
  POLYMER_API_ENDPOINT?: string;
  /** Polymer API token */
  POLYMER_API_TOKEN?: string;
  /** Log level */
  LOG_LEVEL?: string;
}
