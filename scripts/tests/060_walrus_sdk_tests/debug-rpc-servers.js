#!/usr/bin/env node

/**
 * Debug the difference between suibase proxy and direct sui.io calls
 */

import { SuiClient } from '@mysten/sui/client';

async function debugRpcCall(name, url) {
  console.log(`\n=== Testing ${name} (${url}) ===`);
  
  try {
    // Create client
    const client = new SuiClient({ url });
    
    // Intercept fetch to log headers
    const originalFetch = global.fetch;
    global.fetch = async (resource, options) => {
      console.log(`Making request to: ${resource}`);
      console.log('Request headers:', JSON.stringify(options?.headers || {}, null, 2));
      console.log('Request body:', options?.body);
      
      const response = await originalFetch(resource, options);
      console.log('Response status:', response.status);
      console.log('Response headers:');
      for (const [key, value] of response.headers.entries()) {
        console.log(`  ${key}: ${value}`);
      }
      
      // Clone response to read body for inspection
      const clonedResponse = response.clone();
      const text = await clonedResponse.text();
      console.log('Response body (first 200 chars):', text.substring(0, 200));
      
      return response;
    };
    
    // Make the call
    const result = await client.getLatestCheckpointSequenceNumber();
    console.log(`✓ Success: ${result}`);
    
    // Restore original fetch
    global.fetch = originalFetch;
    
  } catch (error) {
    console.log(`✗ Failed: ${error.message}`);
    
    // Restore original fetch on error too
    const originalFetch = global.fetch;
    if (global.fetch !== originalFetch) {
      global.fetch = originalFetch;
    }
  }
}

async function main() {
  // Test both servers
  await debugRpcCall('suibase-proxy', 'http://localhost:44342');
  await debugRpcCall('sui.io', 'https://fullnode.testnet.sui.io:443');
}

main().catch(console.error);