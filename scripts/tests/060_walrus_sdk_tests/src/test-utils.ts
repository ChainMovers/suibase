import { getFullnodeUrl, SuiClient } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';

export interface TestConfig {
  workdir: string;
  relayPort: string;
  relayProxyPort: string;
  secretAccount: string;
}

export interface TestResult {
  success: boolean;
  message: string;
  details?: any;
}

/**
 * Get test configuration from environment variables
 */
export function getTestConfig(): TestConfig {
  const workdir = process.env.WORKDIR || 'testnet';
  const relayPort = process.env.WALRUS_RELAY_PORT;
  const relayProxyPort = process.env.WALRUS_RELAY_PROXY_PORT;
  const secretAccount = process.env.SECRET_TESTNET_ACCOUNT;

  if (!relayPort) {
    throw new Error('WALRUS_RELAY_PORT environment variable is not set');
  }

  if (!relayProxyPort) {
    throw new Error('WALRUS_RELAY_PROXY_PORT environment variable is not set');
  }

  if (!secretAccount) {
    throw new Error('SECRET_TESTNET_ACCOUNT environment variable is not set');
  }

  return {
    workdir,
    relayPort,
    relayProxyPort,
    secretAccount,
  };
}

/**
 * Create a keypair from the secret account (private key or mnemonic)
 */
export function createKeypair(secretAccount: string): Ed25519Keypair {
  try {
    // Try to parse as private key first (hex string)
    if (secretAccount.startsWith('0x') || secretAccount.length === 64) {
      const privateKey = secretAccount.startsWith('0x') 
        ? secretAccount.slice(2) 
        : secretAccount;
      
      // Convert hex string to Uint8Array
      const privateKeyBytes = new Uint8Array(
        privateKey.match(/.{1,2}/g)!.map(byte => parseInt(byte, 16))
      );
      
      return Ed25519Keypair.fromSecretKey(privateKeyBytes);
    }
    
    // Try to parse as mnemonic
    return Ed25519Keypair.deriveKeypair(secretAccount);
  } catch (error) {
    throw new Error(`Failed to create keypair from secret account: ${error instanceof Error ? error.message : String(error)}`);
  }
}

/**
 * Create a configured Sui client for testing
 */
export function createSuiClient(config: TestConfig): SuiClient {
  console.log(`Creating Sui client for ${config.workdir}`);
  
  // Map workdir to network name for getFullnodeUrl
  let networkName: 'mainnet' | 'testnet' | 'devnet' | 'localnet';
  switch (config.workdir) {
    case 'mainnet':
      networkName = 'mainnet';
      break;
    case 'testnet':
      networkName = 'testnet';
      break;
    case 'devnet':
      networkName = 'devnet';
      break;
    case 'localnet':
      networkName = 'localnet';
      break;
    default:
      throw new Error(`Unsupported workdir: ${config.workdir}`);
  }
  
  const client = new SuiClient({
    url: getFullnodeUrl(networkName),
  });

  return client;
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