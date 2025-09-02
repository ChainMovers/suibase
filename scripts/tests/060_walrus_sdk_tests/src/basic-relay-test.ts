#!/usr/bin/env node

import {
  getTestConfig,
  getRelayUrl,
  getRelayProxyUrl,
  validateRelayConnection,
  logResult,
  TestResult,
} from './test-utils.js';

/**
 * Basic test function that validates both relay connections
 */
async function runBasicRelayTest(): Promise<TestResult> {
  console.log('=== Basic Walrus Relay Connection Test ===');
  
  try {
    // Get configuration from environment
    const config = getTestConfig();
    console.log(`Testing with workdir: ${config.workdir}`);
    
    // Test local relay connection
    const relayUrl = getRelayUrl(config);
    console.log(`Validating local relay connection at ${relayUrl}...`);
    
    const relayCheck = await validateRelayConnection(relayUrl);
    if (!relayCheck.success) {
      return {
        success: false,
        message: `Local relay validation failed: ${relayCheck.message}`,
        details: relayCheck.details,
      };
    }
    console.log('✓ Local relay connection successful');

    // Test proxy relay connection
    const relayProxyUrl = getRelayProxyUrl(config);
    console.log(`Validating proxy relay connection at ${relayProxyUrl}...`);
    
    const relayProxyCheck = await validateRelayConnection(relayProxyUrl);
    if (!relayProxyCheck.success) {
      return {
        success: false,
        message: `Proxy relay validation failed: ${relayProxyCheck.message}`,
        details: relayProxyCheck.details,
      };
    }
    console.log('✓ Proxy relay connection successful');
    
    return {
      success: true,
      message: 'Both relay connections validated successfully',
      details: {
        localRelay: {
          url: relayUrl,
          response: relayCheck.details,
        },
        proxyRelay: {
          url: relayProxyUrl,
          response: relayProxyCheck.details,
        },
      },
    };
    
  } catch (error) {
    return {
      success: false,
      message: `Test failed with error: ${error instanceof Error ? error.message : String(error)}`,
      details: {
        error: error instanceof Error ? {
          name: error.name,
          message: error.message,
          stack: error.stack,
        } : error,
      },
    };
  }
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  try {
    const result = await runBasicRelayTest();
    logResult(result);
    
    // Exit with appropriate code for bash wrapper
    process.exit(result.success ? 0 : 1);
  } catch (error) {
    console.error('Fatal error in test execution:', error);
    process.exit(1);
  }
}

// Run the test if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error('Unhandled error:', error);
    process.exit(1);
  });
}