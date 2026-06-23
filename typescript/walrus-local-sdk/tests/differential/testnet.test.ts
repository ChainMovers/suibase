// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

// Cross-environment DIFFERENTIAL suite: runs the SAME operations through the genuine
// `@mysten/walrus` on **testnet** and `@suibase/walrus-local` on **localnet**, asserting
// result parity AND error-shape parity. This is the only test that proves the product
// promise — "everything you can do on testnet you can do on localnet" — against the real
// network, and the only one that catches FIXTURE_BLOB_ID / encoder drift and error-class
// divergence (ERR-1/2/3) before they ship.
//
// OFF BY DEFAULT, and it can ONLY skip — never fail — when funds/network are absent, so
// GitHub Actions (no key, no funds) always lands on a skip and exits 0. Three guards each
// downgrade to skip:
//   (a) WALRUS_TESTNET_DIFF !== "1"            -> whole suite skipped (the default).
//   (b) WALRUS_TESTNET_KEY unset / non-ed25519 -> skipped.
//   (c) RPC unreachable OR SUI gas below a floor OR a per-write funds/network error
//                                              -> skipped with a "fund <address>" message.
// Only genuine parity assertions (blob-id mismatch, error-shape divergence) fail.
//
// Run it (developer with a funded testnet wallet):
//   WALRUS_TESTNET_DIFF=1 WALRUS_TESTNET_KEY=suiprivkey1... npm run test:differential
//
// Requires Node >= 22.

import { test, describe } from "node:test";
import assert from "node:assert/strict";

import { SuiJsonRpcClient } from "@mysten/sui/jsonRpc";
import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";
import { decodeSuiPrivateKey } from "@mysten/sui/cryptography";
import type { Signer } from "@mysten/sui/cryptography";

// Import EVERYTHING the test compares from our barrel — the same names a migrated app
// would use — which also exercises the ERR-1 re-export at runtime.
import {
  BlobNotCertifiedError,
  RetryableWalrusClientError,
  WalrusClient,
  WalrusLocalClient,
  WalrusLocalError,
} from "../../src/index.js";
import { FIXTURE_BLOB_ID, FIXTURE_CONTENT, loadLocalnetSigner, utf8 } from "../helpers.js";

const DIFF = process.env.WALRUS_TESTNET_DIFF === "1";
const TESTNET_RPC = process.env.WALRUS_TESTNET_RPC ?? "https://fullnode.testnet.sui.io:443";
const MIN_SUI_MIST = 100_000_000n; // 0.1 SUI — a floor for gas; below it we skip rather than fail.

let skip: boolean | string = DIFF
  ? false
  : "set WALRUS_TESTNET_DIFF=1 (+ a funded WALRUS_TESTNET_KEY) to run the testnet differential";
let testnet: WalrusClient | undefined;
let local: WalrusLocalClient | undefined;
let testnetSigner: Signer | undefined;
let localSigner: Signer | undefined;

if (DIFF) {
  try {
    const key = process.env.WALRUS_TESTNET_KEY;
    if (!key) {
      skip = "WALRUS_TESTNET_KEY not set — provide a funded testnet ed25519 key (suiprivkey1…) to run";
    } else {
      const parsed = decodeSuiPrivateKey(key);
      if (parsed.scheme !== "ED25519") {
        skip = `WALRUS_TESTNET_KEY is ${parsed.scheme}; this differential supports ED25519 only`;
      } else {
        testnetSigner = Ed25519Keypair.fromSecretKey(parsed.secretKey);
        const suiClient = new SuiJsonRpcClient({ url: TESTNET_RPC, network: "testnet" });
        testnet = new WalrusClient({ network: "testnet", suiClient });
        const address = testnetSigner.toSuiAddress();
        // Read-only gas precheck (free; no signing). Insufficient SUI -> skip, never fail.
        const { totalBalance } = await suiClient.getBalance({ owner: address });
        if (BigInt(totalBalance) < MIN_SUI_MIST) {
          skip = `insufficient testnet SUI for gas at ${address} (have ${totalBalance} MIST) — fund it to run`;
        } else {
          local = new WalrusLocalClient();
          localSigner = loadLocalnetSigner();
          if (!(await local.sbLocalReady())) {
            skip = "localnet + sb-local not available — the differential needs BOTH stacks up";
          }
        }
      }
    }
  } catch (e) {
    // Offline / RPC error / bad key / faucet hiccup — degrade to skip, never fail CI.
    skip = `testnet differential setup skipped (offline / RPC / key): ${(e as Error).message}`;
  }
}

/** Collapse a @mysten/walrus or WalrusLocalError onto a small, comparable parity contract. */
function normalizeWalrusError(err: unknown): { kind: string; retryable: boolean } {
  if (err instanceof BlobNotCertifiedError) {
    return { kind: "not_found", retryable: err instanceof RetryableWalrusClientError };
  }
  if (err instanceof RetryableWalrusClientError) return { kind: "retryable_other", retryable: true };
  if (err instanceof WalrusLocalError) {
    if (err.code === "BLOB_NOT_FOUND") return { kind: "not_found", retryable: false };
    if (err.code === "BAD_REQUEST") return { kind: "bad_request", retryable: false };
    return { kind: err.code.toLowerCase(), retryable: false };
  }
  return { kind: "other", retryable: false };
}

/** Heuristic: an environmental (funds/network) failure we should SKIP on, not fail on. */
function looksEnvironmental(err: unknown): boolean {
  const m = (err instanceof Error ? err.message : String(err)).toLowerCase();
  return /insufficient|no valid gas|balance|fund|gas coin|timeout|timed out|fetch failed|network|econn|enotfound|exchange|rate limit|429|503/.test(
    m,
  );
}

describe("testnet ⇆ localnet differential (genuine @mysten/walrus vs @suibase/walrus-local)", () => {
  // (1) Blob-id parity + status field-set parity — the live proof of FIXTURE_BLOB_ID.
  test("fixture content yields a bit-identical blob id (and matching status shape) on both stacks", { skip }, async (t) => {
    let tn;
    try {
      tn = await testnet!.writeBlob({ blob: utf8(FIXTURE_CONTENT), deletable: false, epochs: 1, signer: testnetSigner! });
    } catch (e) {
      if (looksEnvironmental(e)) return t.skip(`testnet write skipped (funds/network): ${(e as Error).message}`);
      throw e;
    }
    assert.equal(tn.blobId, FIXTURE_BLOB_ID, "testnet blob id drifted from the recorded fixture (encoder change?)");

    const ln = await local!.writeBlob({ blob: utf8(FIXTURE_CONTENT), deletable: false, epochs: 1, signer: localSigner! });
    assert.equal(ln.blobId, FIXTURE_BLOB_ID, "localnet blob id drifted from the recorded fixture");
    assert.equal(ln.blobId, tn.blobId, "localnet and testnet blob ids differ for identical content");

    // Status SHAPE parity: same union variant + same field set (values like statusEvent /
    // initialCertifiedEpoch legitimately differ, so compare key sets, not values).
    const tnStatus = await testnet!.getVerifiedBlobStatus({ blobId: tn.blobId });
    const lnStatus = await local!.getVerifiedBlobStatus({ blobId: ln.blobId });
    assert.equal(lnStatus.type, "permanent");
    assert.equal(lnStatus.type, tnStatus.type, "blob status variant differs across stacks");
    assert.deepEqual(
      Object.keys(lnStatus).sort(),
      Object.keys(tnStatus).sort(),
      "blob status field set differs across stacks",
    );
  });

  // (2) write -> read round-trip parity: identical content, identical bytes back, identical id.
  test("write → read round-trips identically on both stacks", { skip }, async (t) => {
    const payload = utf8(`@suibase/walrus-local differential round-trip ${FIXTURE_BLOB_ID.slice(0, 8)}`);
    let tn;
    try {
      tn = await testnet!.writeBlob({ blob: payload, deletable: false, epochs: 1, signer: testnetSigner! });
    } catch (e) {
      if (looksEnvironmental(e)) return t.skip(`testnet write skipped (funds/network): ${(e as Error).message}`);
      throw e;
    }
    assert.deepEqual(await testnet!.readBlob({ blobId: tn.blobId }), payload, "testnet round-trip bytes differ");

    const ln = await local!.writeBlob({ blob: payload, deletable: false, epochs: 1, signer: localSigner! });
    assert.deepEqual(await local!.readBlob({ blobId: ln.blobId }), payload, "localnet round-trip bytes differ");
    assert.equal(ln.blobId, tn.blobId, "identical content must yield identical blob id across stacks");
  });

  // (3) ERROR parity (the core of ERR-2): a missing-but-valid-format id throws an
  // EQUIVALENT error — the SAME normalized {kind, retryable} — on both stacks.
  test("readBlob(unstored id) throws an equivalent (retryable, not_found) error on both stacks", { skip }, async (t) => {
    const unstored = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"; // valid 32-byte format, never written
    let tnErr: unknown;
    try {
      await testnet!.readBlob({ blobId: unstored });
      assert.fail("testnet readBlob(unstored) unexpectedly resolved");
    } catch (e) {
      if (looksEnvironmental(e)) return t.skip(`testnet read skipped (network): ${(e as Error).message}`);
      tnErr = e;
    }
    let lnErr: unknown;
    try {
      await local!.readBlob({ blobId: unstored });
      assert.fail("localnet readBlob(unstored) unexpectedly resolved");
    } catch (e) {
      lnErr = e;
    }
    const tnNorm = normalizeWalrusError(tnErr);
    const lnNorm = normalizeWalrusError(lnErr);
    assert.deepEqual(
      lnNorm,
      tnNorm,
      `error-shape parity broken: localnet ${JSON.stringify(lnNorm)} != testnet ${JSON.stringify(tnNorm)}`,
    );
    assert.equal(tnNorm.kind, "not_found", "missing blob should normalize to not_found");
    assert.equal(tnNorm.retryable, true, "missing-blob read must be retryable on both stacks");
  });

  // (4) malformed id is rejected on both (weaker: the parse-level rejection path may differ).
  test("readBlob(malformed id) is rejected on both stacks", { skip }, async () => {
    const malformed = "not-a-valid-blob-id";
    await assert.rejects(() => testnet!.readBlob({ blobId: malformed }), "testnet accepted a malformed id");
    await assert.rejects(() => local!.readBlob({ blobId: malformed }), "localnet accepted a malformed id");
  });
});
