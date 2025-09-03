import { SuiClient } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import * as fs from 'fs';
import * as path from 'path';
import * as yaml from 'js-yaml';

export interface TestConfig {
  workdir: string;
  relayPort: string;
  relayProxyPort: string;
}

export interface TestResult {
  success: boolean;
  message: string;
  details?: any;
}

export interface SuiClientConfig {
  keystore: {
    File: string;
  };
  external_keys?: any;
  envs: Array<{
    alias: string;
    rpc: string;
    ws?: string;
    basic_auth?: any;
  }>;
  active_env: string;
  active_address: string;
}

/**
 * Read Suibase client configuration for the given workdir
 */
export function readSuibaseClientConfig(workdir: string): SuiClientConfig {
  const configPath = path.join(process.env.HOME || '~', 'suibase', 'workdirs', workdir, 'config-default', 'client.yaml');
  
  if (!fs.existsSync(configPath)) {
    throw new Error(`Suibase client config not found at: ${configPath}`);
  }
  
  const configData = fs.readFileSync(configPath, 'utf8');
  const config = yaml.load(configData) as SuiClientConfig;
  
  if (!config || !config.envs || !config.active_env) {
    throw new Error(`Invalid client configuration in ${configPath}`);
  }
  
  return config;
}

/**
 * Get test configuration from environment variables
 */
export function getTestConfig(): TestConfig {
  const workdir = process.env.WORKDIR || 'testnet';
  const relayPort = process.env.WALRUS_RELAY_PORT;
  const relayProxyPort = process.env.WALRUS_RELAY_PROXY_PORT;

  if (!relayPort) {
    throw new Error('WALRUS_RELAY_PORT environment variable is not set');
  }

  if (!relayProxyPort) {
    throw new Error('WALRUS_RELAY_PROXY_PORT environment variable is not set');
  }

  return {
    workdir,
    relayPort,
    relayProxyPort,
  };
}

/**
 * Find keypair for active address from Suibase keystore
 */
export function findKeypairForActiveAddress(workdir: string): Ed25519Keypair {
  try {
    // Read Suibase client configuration
    const suibaseConfig = readSuibaseClientConfig(workdir);
    const activeAddress = suibaseConfig.active_address;
    
    console.log(`Looking for keypair for active address: ${activeAddress}`);
    
    // Read the keystore file
    const keystorePath = suibaseConfig.keystore.File;
    if (!fs.existsSync(keystorePath)) {
      throw new Error(`Keystore file not found at: ${keystorePath}`);
    }
    
    const keystoreData = fs.readFileSync(keystorePath, 'utf8');
    const privateKeys = JSON.parse(keystoreData) as string[];
    
    console.log(`Found ${privateKeys.length} private keys in keystore`);
    
    // Try each private key to find the one that matches the active address
    for (let i = 0; i < privateKeys.length; i++) {
      try {
        const privateKeyBase64 = privateKeys[i];
        // Decode from Base64
        const privateKeyBytes = Uint8Array.from(atob(privateKeyBase64), c => c.charCodeAt(0));
        
        // Skip the first byte (scheme flag) and take the next 32 bytes for Ed25519
        const ed25519PrivateKey = privateKeyBytes.slice(1, 33);
        
        const keypair = Ed25519Keypair.fromSecretKey(ed25519PrivateKey);
        const derivedAddress = keypair.toSuiAddress();
        
        console.log(`Keystore entry ${i}: ${derivedAddress}`);
        
        if (derivedAddress === activeAddress) {
          console.log(`✓ Found matching keypair for active address`);
          return keypair;
        }
      } catch (error) {
        // Skip invalid entries
        console.log(`Skipping keystore entry ${i}: ${error instanceof Error ? error.message : String(error)}`);
        continue;
      }
    }
    
    throw new Error(`No keypair found for active address ${activeAddress}`);
  } catch (error) {
    throw new Error(`Failed to find keypair for active address: ${error instanceof Error ? error.message : String(error)}`);
  }
}


/**
 * Test RPC endpoint to see if it works properly
 * Includes retry logic to handle transient failures
 */
async function testRpcEndpoint(url: string): Promise<boolean> {
  const maxRetries = 3;
  const retryDelay = 1000; // 1 second
  
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const testClient = new SuiClient({ url });
      // Simple test call to check if RPC is working
      await testClient.getLatestCheckpointSequenceNumber();
      return true;
    } catch (error) {
      if (attempt === maxRetries) {
        console.log(`RPC endpoint test failed after ${maxRetries} attempts: ${error instanceof Error ? error.message : String(error)}`);
        return false;
      }
      
      // Wait before retry
      console.log(`RPC test attempt ${attempt} failed, retrying in ${retryDelay}ms...`);
      await new Promise(resolve => setTimeout(resolve, retryDelay));
    }
  }
  
  return false;
}

/**
 * Create a configured Sui client for testing using Suibase configuration
 * This function requires a working Suibase setup and will fail if the local RPC proxy is not available
 */
export async function createSuiClient(config: TestConfig): Promise<SuiClient> {
  console.log(`Creating Sui client for ${config.workdir} using Suibase configuration`);
  
  // Read Suibase client configuration (will throw if not available)
  const suibaseConfig = readSuibaseClientConfig(config.workdir);
  
  // Find the active environment
  const activeEnv = suibaseConfig.envs.find(env => env.alias === suibaseConfig.active_env);
  if (!activeEnv) {
    throw new Error(`Active environment '${suibaseConfig.active_env}' not found in client configuration`);
  }
  
  console.log(`Suibase config found - RPC: ${activeEnv.rpc} (${activeEnv.alias})`);
  console.log(`Active address: ${suibaseConfig.active_address}`);
  
  // Test local RPC proxy and require it to be working
  console.log('Testing local Suibase RPC proxy...');
  const localWorks = await testRpcEndpoint(activeEnv.rpc);
  
  if (!localWorks) {
    throw new Error(`Suibase RPC proxy at ${activeEnv.rpc} is not responding. This test requires a working Suibase setup.`);
  }
  
  console.log('✓ Using local Suibase RPC proxy');
  return new SuiClient({
    url: activeEnv.rpc,
  });
}

/**
 * Get the relay URL for testing
 */
export function getRelayUrl(config: TestConfig): string {
  return `http://localhost:${config.relayPort}`;
}

/**
 * Get the relay proxy URL for testing
 */
export function getRelayProxyUrl(config: TestConfig): string {
  return `http://localhost:${config.relayProxyPort}`;
}

/**
 * Create test blob content
 */
export function createTestBlob(): Uint8Array {
  const testData = `Suibase Walrus SDK Test - ${new Date().toISOString()}`;
  return new TextEncoder().encode(testData);
}

/**
 * Log test results in a format that the bash wrapper can parse
 */
export function logResult(result: TestResult): void {
  if (result.success) {
    console.log(`✓ SUCCESS: ${result.message}`);
    if (result.details) {
      console.log(`Details: ${JSON.stringify(result.details, null, 2)}`);
    }
  } else {
    console.error(`✗ FAILURE: ${result.message}`);
    if (result.details) {
      console.error(`Details: ${JSON.stringify(result.details, null, 2)}`);
    }
  }
}

/**
 * Wait for a specified amount of time (useful for testing timing)
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Check SUI and WAL balance for an address with retry logic for rate limiting and RPC issues
 */
export async function checkBalances(suiClient: SuiClient, address: string): Promise<{ suiBalance: number; walBalance: number }> {
  const maxRetries = 5;
  const baseDelay = 1000; // 1 second
  
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      // Get all coins for the address
      const coins = await suiClient.getAllCoins({ owner: address });
      
      let suiBalance = 0;
      let walBalance = 0;
      
      for (const coin of coins.data) {
        if (coin.coinType === '0x2::sui::SUI') {
          suiBalance += parseInt(coin.balance) / 1_000_000_000; // Convert MIST to SUI
        } else if (coin.coinType.includes('::wal::WAL')) {
          walBalance += parseInt(coin.balance) / 1_000_000_000; // Convert to WAL
        }
      }
      
      return { suiBalance, walBalance };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Check for various transient error types
      const isTransientError = error instanceof Error && (
        errorMessage.includes('429') || 
        errorMessage.includes('Too Many Requests') ||
        errorMessage.includes('not valid JSON') ||
        errorMessage.includes('Unexpected token') ||
        errorMessage.includes('fetch failed')
      );
      
      if (isTransientError && attempt < maxRetries) {
        const delay = baseDelay * Math.pow(2, attempt - 1); // Exponential backoff
        console.log(`Balance check attempt ${attempt} failed (${errorMessage}), retrying in ${delay}ms...`);
        await sleep(delay);
        continue;
      }
      
      throw new Error(`Failed to check balances: ${errorMessage}`);
    }
  }
  
  throw new Error('Failed to check balances after all retry attempts');
}

/**
 * Verify blob exists on Walrus using aggregator
 */
export async function verifyBlobOnWalrus(blobId: string): Promise<TestResult> {
  try {
    const aggregatorUrl = `https://aggregator.walrus-testnet.walrus.space/v1/blobs/${blobId}`;
    console.log(`Verifying blob on Walrus aggregator: ${aggregatorUrl}`);
    
    const response = await fetch(aggregatorUrl, {
      method: 'GET',
    });
    
    if (!response.ok) {
      return {
        success: false,
        message: `Blob verification failed: HTTP ${response.status}`,
        details: { 
          status: response.status, 
          statusText: response.statusText,
          blobId,
          aggregatorUrl,
        }
      };
    }
    
    const blobData = await response.arrayBuffer();
    
    return {
      success: true,
      message: 'Blob successfully verified on Walrus',
      details: { 
        blobId,
        blobSize: blobData.byteLength,
        aggregatorUrl,
      }
    };
  } catch (error) {
    return {
      success: false,
      message: 'Failed to verify blob on Walrus',
      details: { 
        blobId,
        error: error instanceof Error ? error.message : String(error) 
      }
    };
  }
}

/**
 * Validate that the relay is responding
 */
export async function validateRelayConnection(relayUrl: string): Promise<TestResult> {
  try {
    const response = await fetch(`${relayUrl}/v1/tip-config`);
    
    if (!response.ok) {
      return {
        success: false,
        message: `Relay returned HTTP ${response.status}`,
        details: { status: response.status, statusText: response.statusText }
      };
    }
    
    const data = await response.json();
    return {
      success: true,
      message: 'Relay is responding correctly',
      details: data
    };
  } catch (error) {
    return {
      success: false,
      message: 'Failed to connect to relay',
      details: { error: error instanceof Error ? error.message : String(error) }
    };
  }
}

/**
 * Interface for RPC proxy statistics
 */
export interface RpcProxyStats {
  summary: {
    successOnFirstAttempt: number;
    failuresTotalCount?: number;
  };
  [key: string]: any;
}

/**
 * Get RPC proxy statistics from suibase daemon
 */
export async function getRpcProxyStats(workdir: string): Promise<RpcProxyStats | null> {
  try {
    const { execSync } = await import('child_process');
    const command = `${process.env.HOME}/suibase/scripts/${workdir} links --json`;
    
    console.log(`Getting RPC stats with command: ${command}`);
    const output = execSync(command, { 
      encoding: 'utf8', 
      timeout: 10000,
      stdio: ['ignore', 'pipe', 'ignore'] // ignore stderr to avoid noise
    });
    
    const statsJson = JSON.parse(output.trim());
    
    if (!statsJson.result || !statsJson.result.summary) {
      throw new Error('Invalid stats JSON structure');
    }
    
    return statsJson.result as RpcProxyStats;
  } catch (error) {
    console.warn(`Failed to get RPC proxy stats: ${error instanceof Error ? error.message : String(error)}`);
    return null;
  }
}

/**
 * Compare RPC proxy stats to verify usage
 */
export function verifyRpcProxyUsage(initialStats: RpcProxyStats | null, finalStats: RpcProxyStats | null): TestResult {
  if (!initialStats || !finalStats) {
    return {
      success: false,
      message: 'Cannot verify RPC proxy usage - stats not available',
      details: {
        initialStats: initialStats !== null,
        finalStats: finalStats !== null
      }
    };
  }
  
  const initialSuccess = initialStats.summary.successOnFirstAttempt || 0;
  const finalSuccess = finalStats.summary.successOnFirstAttempt || 0;
  const increment = finalSuccess - initialSuccess;
  
  console.log(`RPC proxy stats - Initial: ${initialSuccess}, Final: ${finalSuccess}, Increment: ${increment}`);
  
  if (increment > 0) {
    return {
      success: true,
      message: `RPC proxy usage verified (${increment} successful requests)`,
      details: {
        initialSuccess,
        finalSuccess,
        increment,
        message: 'Suibase RPC proxy is being used for both Walrus relay and Sui network calls'
      }
    };
  } else {
    return {
      success: false,
      message: 'No RPC proxy usage detected',
      details: {
        initialSuccess,
        finalSuccess,
        increment,
        message: 'Upload operations may not be using the suibase-daemon RPC proxy'
      }
    };
  }
}