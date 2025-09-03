#!/usr/bin/env node

/**
 * Test if the Content-Type header fix resolves the SuiClient issue
 */

import { SuiClient } from '@mysten/sui/client';

async function testContentTypeFix() {
  console.log("=== Testing Content-Type Header Fix ===\n");
  
  const servers = {
    'suibase-proxy': 'http://localhost:44342',
    'sui.io': 'https://fullnode.testnet.sui.io:443'
  };
  
  for (const [name, url] of Object.entries(servers)) {
    console.log(`Testing ${name} (${url}):`);
    
    try {
      const client = new SuiClient({ url });
      
      // Test 1: Basic call
      console.log("  1. Testing getLatestCheckpointSequenceNumber...");
      const checkpoint = await client.getLatestCheckpointSequenceNumber();
      console.log(`     ✓ Success: ${checkpoint}`);
      
      // Test 2: Balance check (the failing operation)
      console.log("  2. Testing getAllCoins (balance check)...");
      const coins = await client.getAllCoins({ 
        owner: '0xdf3c05624de3a581b31c48e07ff4bee64c1c480f064f75739fdcde7fb752075f' 
      });
      console.log(`     ✓ Success: Found ${coins.data.length} coins`);
      
      // Test 3: Complex operation
      console.log("  3. Testing multiGetObjects...");
      const objects = await client.multiGetObjects(['0x6'], { showContent: true });
      console.log(`     ✓ Success: Got ${objects.length} objects`);
      
      console.log(`  → ${name}: ALL TESTS PASSED!\n`);
      
    } catch (error) {
      console.log(`     ✗ Failed: ${error.message}`);
      if (error.message.includes('not valid JSON')) {
        console.log(`     🔍 Still getting binary response issue!`);
      }
      console.log(`  → ${name}: FAILED\n`);
    }
  }
}

testContentTypeFix().catch(console.error);