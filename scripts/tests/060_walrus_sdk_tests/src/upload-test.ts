#!/usr/bin/env node

import {
  getTestConfig,
  findKeypairForActiveAddress,
  createSuiClient,
  getRelayUrl,
  getRelayProxyUrl,
  createTestBlob,
  logResult,
  validateRelayConnection,
  verifyBlobOnWalrus,
  checkBalances,
  getRpcProxyStats,
  verifyRpcProxyUsage,
  TestResult,
} from "./test-utils.js";

/**
 * Main test function that verifies Walrus SDK upload functionality
 */
async function runWalrusUploadTest(): Promise<TestResult> {
  console.log("=== Walrus SDK Upload Test ===");

  try {
    // Get configuration from environment
    const config = getTestConfig();
    console.log(`Testing with workdir: ${config.workdir}`);

    // Validate relay connection first
    const relayUrl = getRelayUrl(config);
    console.log(`Validating relay connection at ${relayUrl}...`);

    const relayCheck = await validateRelayConnection(relayUrl);
    if (!relayCheck.success) {
      return {
        success: false,
        message: `Relay validation failed: ${relayCheck.message}`,
        details: relayCheck.details,
      };
    }
    console.log("✓ Relay is responding correctly");

    // Find keypair for the active address from Suibase keystore
    console.log("Finding keypair for active address from Suibase keystore...");
    let keypair;
    let address;

    try {
      keypair = findKeypairForActiveAddress(config.workdir);
      address = keypair.toSuiAddress();
      console.log(`✓ Found active address keypair: ${address}`);
    } catch (error) {
      console.log(`SKIP: Cannot retrieve active address: ${error instanceof Error ? error.message : String(error)}`);
      process.exit(2);
    }

    const suiClient = await createSuiClient(config);

    // Check SUI and WAL balances BEFORE capturing initial RPC stats
    console.log("Checking SUI and WAL balances...");
    try {
      const { suiBalance, walBalance } = await checkBalances(suiClient, address);
      console.log(`SUI balance: ${suiBalance.toFixed(6)}`);
      console.log(`WAL balance: ${walBalance.toFixed(6)}`);

      const minBalance = 0.05;
      if (suiBalance < minBalance) {
        console.log(`SKIP: Insufficient SUI balance (${suiBalance.toFixed(6)} < ${minBalance})`);
        process.exit(2);
      }

      if (walBalance < minBalance) {
        console.log(`SKIP: Insufficient WAL balance (${walBalance.toFixed(6)} < ${minBalance})`);
        process.exit(2);
      }

      console.log("✓ Sufficient SUI and WAL balances for testing");
    } catch (error) {
      console.log(`SKIP: Cannot check balances: ${error instanceof Error ? error.message : String(error)}`);
      process.exit(2);
    }

    const blobData = createTestBlob();
    console.log(`✓ Created test blob (${blobData.length} bytes)`);

    // Get initial RPC proxy stats AFTER all setup operations
    // This ensures we only measure the walrus SDK operation itself
    console.log("Getting initial RPC proxy statistics...");
    const initialStats = await getRpcProxyStats(config.workdir);
    if (initialStats) {
      console.log(`Initial RPC proxy success count: ${initialStats.summary.successOnFirstAttempt}`);
    } else {
      console.log("Warning: Could not retrieve initial RPC proxy statistics");
    }

    // Now attempt actual Walrus SDK upload through upload relay
    console.log("Attempting Walrus SDK upload through upload relay...");

    try {
      // Import Walrus SDK
      const { WalrusClient } = await import("@mysten/walrus");

      // Create standalone WalrusClient with upload relay configuration
      const relayUrl = getRelayProxyUrl(config);
      console.log(`Using upload relay proxy: ${relayUrl}`);

      const walrusClient = new WalrusClient({
        suiClient,
        network: "testnet",
        uploadRelay: {
          host: relayUrl,
          // No tip configuration since local relay returns "no_tip"
        },
      });

      console.log("✓ Created Walrus client with upload relay configuration");

      // Use the keypair we created earlier as the signer
      const signer = keypair;
      console.log(`Using signer: ${signer.toSuiAddress()}`);

      // Upload the blob using Walrus SDK through upload relay with retry logic
      console.log("Uploading blob via Walrus SDK...");
      const maxRetries = 3;
      const baseDelay = 2000; // 2 seconds
      let uploadResult;

      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
          uploadResult = await walrusClient.writeBlob({
            blob: blobData,
            deletable: true,
            epochs: 3,
            signer: signer,
          });
          break; // Success, exit retry loop
        } catch (retryError) {
          // Check if it's a rate limiting error
          const isRateLimited = retryError instanceof Error &&
            (retryError.message.includes('429') || retryError.message.includes('Too Many Requests'));

          if (isRateLimited && attempt < maxRetries) {
            const delay = baseDelay * Math.pow(2, attempt - 1); // Exponential backoff
            console.log(`Rate limited during upload, retrying in ${delay}ms (attempt ${attempt}/${maxRetries})...`);
            await new Promise(resolve => setTimeout(resolve, delay));
            continue;
          }

          // If not rate limited or max retries exceeded, throw the error
          throw retryError;
        }
      }

      if (!uploadResult) {
        throw new Error("Upload failed after all retry attempts");
      }

      console.log("✓ Successfully uploaded blob to Walrus via SDK and relay");
      console.log(`Blob ID: ${uploadResult.blobId}`);
      console.log(`Blob Object ID: ${uploadResult.blobObject}`);

      // Get final RPC proxy stats IMMEDIATELY after upload and verify usage
      // This ensures we only measure the walrus SDK upload operation
      console.log("Verifying RPC proxy usage for upload operation...");
      const finalStats = await getRpcProxyStats(config.workdir);
      const rpcProxyVerification = verifyRpcProxyUsage(initialStats, finalStats);
      
      if (rpcProxyVerification.success) {
        console.log(`✓ ${rpcProxyVerification.message}`);
        console.log("✓ Suibase daemon is proxying both walrus relay AND sui network JSON-RPC calls");
      } else {
        console.log(`⚠ ${rpcProxyVerification.message}`);
        console.log("⚠ Upload operations may not be using the suibase-daemon RPC proxy");
      }

      // Verify blob exists on Walrus using aggregator (done after RPC stats check)
      console.log("Verifying blob on Walrus...");
      const verificationResult = await verifyBlobOnWalrus(uploadResult.blobId);

      if (verificationResult.success) {
        console.log("✓ Blob successfully verified on Walrus");
        console.log(
          `Verified blob size: ${verificationResult.details?.blobSize} bytes`
        );
      } else {
        console.log("✗ Blob verification failed");
        console.log(`Verification error: ${verificationResult.message}`);
      }

      return {
        success: true,
        message:
          "Walrus SDK upload test completed successfully via upload relay",
        details: {
          relayUrl: relayUrl,
          address: address,
          blobSize: blobData.length,
          blobId: uploadResult.blobId,
          blobObjectId: uploadResult.blobObject,
          epochs: 1,
          usedSigner: signer.toSuiAddress(),
          verification: verificationResult,
          rpcProxyVerification: rpcProxyVerification,
        },
      };
    } catch (uploadError) {
      console.error("✗ Walrus SDK upload failed:", uploadError);
      return {
        success: false,
        message: `Walrus SDK upload failed: ${
          uploadError instanceof Error
            ? uploadError.message
            : String(uploadError)
        }`,
        details: {
          relayUrl: getRelayProxyUrl(config),
          address: address,
          blobSize: blobData.length,
          error:
            uploadError instanceof Error
              ? {
                  name: uploadError.name,
                  message: uploadError.message,
                  stack: uploadError.stack,
                }
              : uploadError,
        },
      };
    }
  } catch (error) {
    return {
      success: false,
      message: `Test failed with error: ${
        error instanceof Error ? error.message : String(error)
      }`,
      details: {
        error:
          error instanceof Error
            ? {
                name: error.name,
                message: error.message,
                stack: error.stack,
              }
            : error,
      },
    };
  }
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  try {
    const result = await runWalrusUploadTest();
    logResult(result);

    // Exit with appropriate code for bash wrapper
    process.exit(result.success ? 0 : 1);
  } catch (error) {
    console.error("Fatal error in test execution:", error);
    process.exit(1);
  }
}

// Run the test if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error("Unhandled error:", error);
    process.exit(1);
  });
}
