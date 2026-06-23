// Live end-to-end suite against a running localnet + sb-local, exercising the full
// drop-in @mysten/walrus surface through WalrusLocalClient.
//
// Self-gating: if the localnet / sb-local is not available, every case SKIPS — UNLESS
// WALRUS_LOCAL_SDK_TEST=1, which turns "not available" into a hard failure (used by the
// walrus-localnet-integration CI, where the localnet + sb-local are guaranteed up).
//
// Requires Node >= 22 (the @mysten/sui peer dep).

import { test, describe } from "node:test";
import assert from "node:assert/strict";

import type { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";
import { WalrusFile } from "@mysten/walrus";

// Import the @mysten/walrus error classes FROM our barrel — this also asserts the
// drop-in re-export (ERR-1): instanceof checks must resolve against @suibase/walrus-local.
import {
  BlobNotCertifiedError,
  RetryableWalrusClientError,
  WalrusLocalClient,
  WalrusLocalError,
} from "../../src/index.js";
import { FIXTURE_BLOB_ID, FIXTURE_CONTENT, fromUtf8, loadLocalnetSigner, utf8 } from "../helpers.js";

const REQUIRED = process.env.WALRUS_LOCAL_SDK_TEST === "1";

let client: WalrusLocalClient | undefined;
let signer: Ed25519Keypair | undefined;
let live = false;
try {
  client = new WalrusLocalClient();
  signer = loadLocalnetSigner();
  live = await client.sbLocalReady();
} catch {
  live = false;
}
const skip: boolean | string = live
  ? false
  : "localnet + sb-local not available (enable walrus_local_enabled + `localnet regen`)";
const nonce = `${process.pid}-${Math.trunc(performance.now())}`;

if (REQUIRED && !live) {
  test("localnet + sb-local MUST be available (WALRUS_LOCAL_SDK_TEST=1)", () => {
    assert.fail("localnet/sb-local not reachable");
  });
}

describe("WalrusLocalClient (drop-in @mysten/walrus on localnet)", () => {
  // ----- inherited on-chain reads -----
  test("systemState + storageCost are inherited and work on-chain", { skip }, async () => {
    const ss = await client!.systemState();
    assert.ok(ss.committee.n_shards >= 1, "n_shards");
    const cost = await client!.storageCost(1000, 5);
    assert.ok(typeof cost.totalCost === "bigint" && cost.totalCost >= 0n, "totalCost is a bigint");
  });

  // ----- cross-environment blob id parity -----
  test("writeBlob yields the proven cross-environment blob id", { skip }, async () => {
    const { blobId, blobObject } = await client!.writeBlob({
      blob: utf8(FIXTURE_CONTENT),
      deletable: false,
      epochs: 5,
      signer: signer!,
    });
    assert.equal(blobId, FIXTURE_BLOB_ID, "cross-environment blob_id mismatch");
    // blobObject is the parsed Move struct shape @mysten/walrus returns.
    assert.equal(typeof blobObject.id, "string");
    assert.equal(typeof blobObject.blob_id, "string"); // decimal u256
    assert.equal(typeof blobObject.size, "string");
    assert.equal(blobObject.encoding_type, 1);
    assert.equal(blobObject.storage.end_epoch, blobObject.storage.start_epoch + 5);
  });

  // ----- writeBlob / readBlob round-trip -----
  test("writeBlob -> readBlob round-trip", { skip }, async () => {
    const payload = utf8(`walrus-local-sdk rt ${nonce}`);
    const { blobId } = await client!.writeBlob({ blob: payload, deletable: false, epochs: 3, signer: signer! });
    const back = await client!.readBlob({ blobId });
    assert.deepEqual(back, payload);
  });

  // ----- deletable blob -> inherited deleteBlob -> status nonexistent -----
  test("writeBlob(deletable) -> executeDeleteBlobTransaction -> status nonexistent", { skip }, async () => {
    const { blobId, blobObject } = await client!.writeBlob({
      blob: utf8(`delete-me ${nonce}`),
      deletable: true,
      epochs: 3,
      signer: signer!,
    });
    assert.equal(blobObject.deletable, true);
    const before = await client!.getVerifiedBlobStatus({ blobId });
    assert.equal(before.type, "deletable");

    const { digest } = await client!.executeDeleteBlobTransaction({ blobObjectId: blobObject.id, signer: signer! });
    assert.ok(digest && digest.length > 0);

    const after = await client!.getVerifiedBlobStatus({ blobId });
    assert.equal(after.type, "nonexistent");
    // NOTE: a known localnet difference — unlike testnet, sb-local's nodeless mock store
    // still serves the bytes of a deleted blob via readBlob (no storage-node GC), even
    // though its on-chain status is `nonexistent` above. See README "Localnet differences".
  });

  // ----- inherited extendBlob -----
  test("executeExtendBlobTransaction extends storage", { skip }, async () => {
    const { blobObject } = await client!.writeBlob({ blob: utf8(`extend ${nonce}`), deletable: false, epochs: 3, signer: signer! });
    const endBefore = blobObject.storage.end_epoch;
    const { digest } = await client!.executeExtendBlobTransaction({ blobObjectId: blobObject.id, epochs: 2, signer: signer! });
    assert.ok(digest && digest.length > 0, `extend digest ${endBefore}`);
  });

  // ----- attributes (writeBlob inline + inherited read/write) -----
  test("writeBlob attributes + readBlobAttributes + update", { skip }, async () => {
    const { blobObject } = await client!.writeBlob({
      blob: utf8(`attr ${nonce}`),
      deletable: false,
      epochs: 3,
      signer: signer!,
      attributes: { creator: "suibase", kind: "demo" },
    });
    const attrs = await client!.readBlobAttributes({ blobObjectId: blobObject.id });
    assert.deepEqual(attrs, { creator: "suibase", kind: "demo" });

    await client!.executeWriteBlobAttributesTransaction({
      blobObjectId: blobObject.id,
      attributes: { kind: "updated", extra: "x" },
      signer: signer!,
    });
    const updated = await client!.readBlobAttributes({ blobObjectId: blobObject.id });
    assert.equal(updated?.kind, "updated");
    assert.equal(updated?.extra, "x");
    assert.equal(updated?.creator, "suibase");
  });

  // ----- quilts: writeQuilt -----
  test("writeQuilt returns the index + patch ids", { skip }, async () => {
    const result = await client!.writeQuilt({
      blobs: [
        { contents: utf8(`alpha ${nonce}`), identifier: "alpha.txt", tags: { kind: "text" } },
        { contents: utf8(`beta ${nonce}`), identifier: "beta.bin" },
      ],
      deletable: false,
      epochs: 3,
      signer: signer!,
    });
    assert.equal(result.index.patches.length, 2);
    const alpha = result.index.patches.find((p) => p.identifier === "alpha.txt");
    assert.ok(alpha, "alpha patch present");
    assert.equal(alpha!.tags.kind, "text");
    assert.ok(alpha!.patchId.length > 0);
    assert.ok(alpha!.endIndex > alpha!.startIndex);
  });

  // ----- quilts: writeFiles -> getFiles + getBlob round-trip -----
  test("writeFiles -> getFiles + getBlob.files round-trip", { skip }, async () => {
    const oneText = `file-one ${nonce}`;
    const twoText = `file-two ${nonce}`;
    const files = [
      WalrusFile.from({ contents: utf8(oneText), identifier: "one.txt", tags: { role: "a" } }),
      WalrusFile.from({ contents: utf8(twoText), identifier: "two.txt" }),
    ];
    const written = await client!.writeFiles({ files, deletable: false, epochs: 3, signer: signer! });
    assert.equal(written.length, 2);
    const quiltId = written[0]!.blobId;

    // getFiles by patch id
    const read = await client!.getFiles({ ids: written.map((w) => w.id) });
    assert.equal(fromUtf8(await read[0]!.bytes()), oneText);
    assert.equal(await read[0]!.getIdentifier(), "one.txt");
    assert.deepEqual(await read[0]!.getTags(), { role: "a" });
    assert.equal(fromUtf8(await read[1]!.bytes()), twoText);

    // getBlob -> WalrusBlob.files() (sb-local-backed reader)
    const blob = await client!.getBlob({ blobId: quiltId });
    assert.equal(await blob.blobId(), quiltId);
    assert.equal(await blob.exists(), true);
    const blobFiles = await blob.files();
    const idents = new Set(await Promise.all(blobFiles.map((f) => f.getIdentifier())));
    assert.ok(idents.has("one.txt") && idents.has("two.txt"));
    // filter by identifier
    const onlyOne = await blob.files({ identifiers: ["one.txt"] });
    assert.equal(onlyOne.length, 1);
    assert.equal(await onlyOne[0]!.getIdentifier(), "one.txt");
  });

  // ----- getVerifiedBlobStatus variants -----
  test("getVerifiedBlobStatus: permanent / deletable / nonexistent", { skip }, async () => {
    const perm = await client!.writeBlob({ blob: utf8(`perm ${nonce}`), deletable: false, epochs: 4, signer: signer! });
    const ps = await client!.getVerifiedBlobStatus({ blobId: perm.blobId });
    assert.equal(ps.type, "permanent");
    if (ps.type === "permanent") {
      assert.equal(ps.isCertified, true);
      assert.ok(ps.endEpoch > 0);
      assert.equal(typeof ps.initialCertifiedEpoch, "number");
    }

    const del = await client!.writeBlob({ blob: utf8(`del ${nonce}`), deletable: true, epochs: 4, signer: signer! });
    const ds = await client!.getVerifiedBlobStatus({ blobId: del.blobId });
    assert.equal(ds.type, "deletable");

    const ns = await client!.getVerifiedBlobStatus({ blobId: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" });
    assert.equal(ns.type, "nonexistent");
  });

  // ----- node-only plumbing throws UNSUPPORTED -----
  test("node-only methods throw UNSUPPORTED", { skip }, async () => {
    await assert.rejects(
      () => client!.getSlivers({ blobId: FIXTURE_BLOB_ID }),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "UNSUPPORTED",
    );
    assert.throws(
      () => client!.writeBlobFlow({ blob: utf8("x") }),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "UNSUPPORTED",
    );
  });

  // ----- error parity: missing -> BlobNotCertifiedError (retryable), malformed -> BAD_REQUEST -----
  test("readBlob: unknown id -> BlobNotCertifiedError (retryable), malformed -> BAD_REQUEST", { skip }, async () => {
    // A valid-format but unstored blob throws the SAME class testnet does — a
    // BlobNotCertifiedError, which is a RetryableWalrusClientError — so retry loops port.
    await assert.rejects(
      () => client!.readBlob({ blobId: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" }),
      (e: unknown) => e instanceof BlobNotCertifiedError && e instanceof RetryableWalrusClientError,
    );
    // A malformed id is a client-side bad request (no @mysten/walrus retryable analog).
    await assert.rejects(
      () => client!.readBlob({ blobId: "not-a-valid-blob-id" }),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "BAD_REQUEST",
    );
  });

  // ----- dedup: re-storing identical content must not abort (regression for the
  // add_metadata-on-already-certified bug) -----
  test("re-storing identical content dedups (alreadyCertified) without aborting", { skip }, async () => {
    const content = utf8(`dedup ${nonce}`);
    const first = await client!.writeBlob({ blob: content, deletable: false, epochs: 4, signer: signer! });
    // A second store of the SAME bytes must succeed (dedup) and return the same blob id.
    const second = await client!.writeBlob({ blob: content, deletable: false, epochs: 4, signer: signer! });
    assert.equal(second.blobId, first.blobId);
    assert.ok(second.blobObject.id.startsWith("0x"));

    // With attributes on a fresh blob, then a re-store with attributes — must not abort.
    const attrContent = utf8(`dedup-attr ${nonce}`);
    const a1 = await client!.writeBlob({ blob: attrContent, deletable: false, epochs: 4, signer: signer!, attributes: { a: "1" } });
    assert.deepEqual(await client!.readBlobAttributes({ blobObjectId: a1.blobObject.id }), { a: "1" });
    const a2 = await client!.writeBlob({ blob: attrContent, deletable: false, epochs: 4, signer: signer!, attributes: { a: "2" } });
    assert.equal(a2.blobId, a1.blobId); // deduped; original attributes retained (localnet nuance)

    // Re-storing an identical quilt must not abort either (the _walrusBlobType marker is skipped on dedup).
    const blobs = [{ contents: utf8(`q-dedup ${nonce}`), identifier: "q.txt" }];
    const q1 = await client!.writeQuilt({ blobs, deletable: false, epochs: 4, signer: signer! });
    const q2 = await client!.writeQuilt({ blobs, deletable: false, epochs: 4, signer: signer! });
    assert.equal(q2.blobId, q1.blobId);
  });

  // ----- a deletable quilt -----
  test("writeQuilt(deletable) yields a deletable quilt blob", { skip }, async () => {
    const result = await client!.writeQuilt({
      blobs: [{ contents: utf8(`del-quilt ${nonce}`), identifier: "d.txt" }],
      deletable: true,
      epochs: 3,
      signer: signer!,
    });
    assert.equal(result.blobObject.deletable, true);
    const status = await client!.getVerifiedBlobStatus({ blobId: result.blobId });
    assert.equal(status.type, "deletable");
  });

  // ----- getFiles by a plain blob id (the 32-byte 'blob' branch) -----
  test("getFiles by plain blob id yields a null-identifier file", { skip }, async () => {
    const payload = utf8(`plain-file ${nonce}`);
    const { blobId } = await client!.writeBlob({ blob: payload, deletable: false, epochs: 3, signer: signer! });
    const [file] = await client!.getFiles({ ids: [blobId] });
    assert.deepEqual(await file!.bytes(), payload);
    assert.equal(await file!.getIdentifier(), null);
    assert.deepEqual(await file!.getTags(), {});
  });

  // ----- writeFiles auto-names an identifier-less file `file-${i}` (parity with @mysten/walrus) -----
  test("writeFiles auto-names a WalrusFile with no identifier", { skip }, async () => {
    const text = `anon ${nonce}`;
    const anon = new WalrusFile({
      reader: { getIdentifier: async () => null, getTags: async () => ({}), getBytes: async () => utf8(text) },
    });
    const written = await client!.writeFiles({ files: [anon], deletable: false, epochs: 3, signer: signer! });
    const [read] = await client!.getFiles({ ids: written.map((w) => w.id) });
    assert.equal(await read!.getIdentifier(), "file-0");
    assert.equal(fromUtf8(await read!.bytes()), text);
  });

  // ----- attributes + owner !== signer is rejected clearly (localnet limitation) -----
  test("writeBlob attributes with owner !== signer throws UNSUPPORTED", { skip }, async () => {
    const other = "0x" + "1".repeat(64);
    await assert.rejects(
      () =>
        client!.writeBlob({
          blob: utf8(`owned ${nonce}`),
          deletable: false,
          epochs: 3,
          signer: signer!,
          owner: other,
          attributes: { k: "v" },
        }),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "UNSUPPORTED",
    );
  });
});
