# @suibase/walrus-local

A **localnet-only drop-in for [`@mysten/walrus`](https://www.npmjs.com/package/@mysten/walrus)**.

`WalrusLocalClient` **extends the real `WalrusClient`**, constructed against the Walrus
package that [Suibase](https://suibase.io) deploys on its localnet. Every **on-chain**
operation is inherited unchanged (so it really runs against the localnet chain), and only
the **node-talking** operations are overridden to go through Suibase's nodeless `sb-local`
HTTP server — **no storage nodes, no funds, no internet**.

Because it *is* a `WalrusClient`, the API and method signatures are **identical** to
`@mysten/walrus`. Code you write against localnet runs verbatim on testnet/mainnet — there
you just construct the genuine `WalrusClient` (with `network: 'testnet'`) instead. Blob ids
are bit-identical across environments, so what you test on localnet behaves the same in
production.

## Requirements

- **Node.js ≥ 22** (the `@mysten/sui` peer dependency requires it).
- Peer deps: `@mysten/walrus` and `@mysten/sui` (install them alongside this package).
- A Suibase localnet with Walrus enabled — in `~/suibase/workdirs/localnet/suibase.yaml`:
  ```yaml
  walrus_local_enabled: true
  ```
  then `localnet regen`. Confirm with `localnet status` (a `Walrus API` line) or
  `curl http://localhost:45840/status`.

## Usage

```ts
// Single-import drop-in: this package re-exports the FULL @mysten/walrus surface
// (WalrusFile, WalrusBlob, the error classes, …) alongside WalrusLocalClient.
import { WalrusLocalClient, WalrusFile } from "@suibase/walrus-local";

// Localnet defaults: Sui RPC http://127.0.0.1:9000, sb-local from suibase.yaml (45840),
// package config read from the deploy descriptor. No network/workdir switch.
const client = new WalrusLocalClient();
// (or pass your own: new WalrusLocalClient({ suiClient, aggregatorUrl, suiRpcUrl }))

// --- blobs (node-talking -> sb-local) ---
const { blobId, blobObject } = await client.writeBlob({ blob, deletable: true, epochs: 5, signer });
const bytes = await client.readBlob({ blobId });

// --- on-chain lifecycle (inherited from WalrusClient, runs against the localnet chain) ---
await client.executeExtendBlobTransaction({ blobObjectId: blobObject.id, epochs: 3, signer });
await client.executeWriteBlobAttributesTransaction({ blobObjectId: blobObject.id, attributes: { k: "v" }, signer });
await client.readBlobAttributes({ blobObjectId: blobObject.id });
await client.executeDeleteBlobTransaction({ blobObjectId: blobObject.id, signer });
await client.storageCost(1000, 5);
const status = await client.getVerifiedBlobStatus({ blobId }); // { type: 'permanent' | 'deletable' | 'nonexistent', … }

// --- quilts / files (node-talking -> sb-local) ---
const written = await client.writeFiles({
  files: [WalrusFile.from({ contents, identifier: "a.txt", tags: { kind: "text" } })],
  deletable: false, epochs: 3, signer,
});
const files = await client.getFiles({ ids: written.map((w) => w.id) });
const blob = await client.getBlob({ blobId: written[0].blobId });
await blob.files();
```

The `signer` is any `@mysten/sui` `Signer` (e.g. an `Ed25519Keypair`) for an address funded
on the localnet — typically the active address from `~/suibase/workdirs/localnet/config`.

## What works, and what doesn't

**Inherited (on-chain, unchanged):** `systemState`, `stakingState`, `storageCost`,
`createStorage*`, `registerBlob*`, `certifyBlob*`, `deleteBlob*`, `extendBlob*`,
`readBlobAttributes` / `writeBlobAttributes*`, `getBlobType`, `reset`.

**Overridden → sb-local (nodeless):** `writeBlob`, `readBlob`, `writeFiles`, `writeQuilt`,
`getFiles`, `getBlob`, `getVerifiedBlobStatus`.

**Not supported (throws `WalrusLocalError` code `UNSUPPORTED`):** the inherently
storage-node plumbing — `getSlivers`, `getSecondarySliver`, `getBlobMetadata`, `writeSliver`,
`writeEncodedBlobToNodes`, `writeBlobToUploadRelay`, `writeBlobFlow`, `writeFilesFlow`, … No
application calls these directly; on a nodeless localnet they have no meaning.

### Errors are drop-in with `@mysten/walrus`

Because the full `@mysten/walrus` surface is re-exported, every error **class** is importable
from this package and `instanceof` checks resolve correctly:

```ts
import { WalrusLocalClient, BlobNotCertifiedError, RetryableWalrusClientError } from "@suibase/walrus-local";

try {
  await client.readBlob({ blobId });
} catch (e) {
  if (e instanceof RetryableWalrusClientError) { /* retry — works on localnet AND testnet */ }
}
```

- A read of a **missing/uncertified blob** (`readBlob`, `getBlob().getBytes()`, `getFiles` on a
  plain blob id) throws the real **`BlobNotCertifiedError`** (a `RetryableWalrusClientError`) —
  the exact class and message testnet throws — so retry-on-retryable loops port verbatim.
- Localnet-specific failures throw **`WalrusLocalError`** with a `code` (`BAD_REQUEST`,
  `QUILT_PATCH_NOT_FOUND`, `SERVER_UNREACHABLE`, `UNSUPPORTED`, …). sb-local's HTTP errors use
  the **same wire envelope** as the real Walrus aggregator/publisher — the Google-API `Status`
  struct with the machine-readable reason at `error.details[0].reason` — so any aggregator/
  publisher client parses them identically.

## Localnet differences (vs testnet/mainnet)

The API is identical, but a few behaviors differ because localnet is nodeless and sb-local
is content-addressed. These rarely matter in practice but are worth knowing:

- **Content dedup.** Re-storing identical bytes returns the *existing* certified Blob (the
  wire is `alreadyCertified`), so `blobObject.id` is **reused** — on testnet each `writeBlob`
  mints a fresh object. A deduped store keeps its original `attributes` (it is the same
  object); attributes from the second call are not re-applied.
- **Read after delete.** After `executeDeleteBlobTransaction`, `getVerifiedBlobStatus`
  correctly reports `nonexistent`, but `readBlob` may still return the cached bytes from
  sb-local's disk (testnet returns not-found). Don't rely on read-after-delete failing.
- **`attributes` + `owner`.** Attributes are written *after* the blob is transferred to
  `owner`, by the `signer`, so `owner` must equal the signer's address. `writeBlob`/
  `writeQuilt`/`writeFiles` with `attributes` **and** a third-party `owner` throw
  `UNSUPPORTED` on localnet (on testnet attributes are set in the register tx before transfer).

## Development

```bash
npm install
npm run typecheck
npm run build
npm run test:unit          # pure tests, no server (Node 22)
npm test                   # unit + live integration (auto-skips if localnet is down)
npm run test:differential  # opt-in: compares against REAL testnet (needs funds; see below)
```

The integration suite self-skips when the localnet / sb-local is unreachable; set
`WALRUS_LOCAL_SDK_TEST=1` to make "not available" a hard failure (used by the
`walrus-localnet-integration` CI). The Rust counterpart is
[`walrus-local-sdk`](https://github.com/ChainMovers/suibase/tree/main/rust/walrus-local-sdk).

**Testnet differential** (`tests/differential/`) proves cross-environment parity for real:
it runs the same write / read / status / error operations through the genuine `@mysten/walrus`
on **testnet** and this client on **localnet**, asserting blob-id equality *and* error-shape
equivalence (via a `normalizeWalrusError()` contract). It is **off by default** and only ever
*skips* — never fails — when the flag is unset, the key is missing, the network is down, or the
wallet is unfunded, so CI stays green with no secrets. Run it with a funded testnet wallet:

```bash
WALRUS_TESTNET_DIFF=1 WALRUS_TESTNET_KEY=suiprivkey1… npm run test:differential
```

## License

Apache-2.0
