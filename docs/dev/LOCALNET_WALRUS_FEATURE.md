# Nodeless Local Walrus + Workdir-Aware `WalrusStore`

## Summary

Suibase can stand up a **nodeless local Walrus** on its localnet: real Walrus
`Blob` / `Storage` / `StoragePool` objects are created on the localnet Sui via genuine
PTBs against genuinely-published Walrus Move packages, blobs are **certified** with a
real BLS certificate, and **WAL is really paid** — but there are **no storage nodes, no
RocksDB, and no erasure coding**. Blob bytes are served from the local filesystem,
keyed by their `blob_id`.

The single operation that normally requires a live storage node — `certify_blob` — is
satisfied **off-node**: the deploy holds the N=1 committee's BLS secret key and
self-signs the confirmation. This is sound because `certify_blob` is a pure on-chain
BLS-aggregate check with no node liveness involved, and a one-member committee is a
trivial quorum (see [the held-key model](#why-nodeless-certify-works)).

On top of this sits **`WalrusStore`**, a workdir-aware Rust client with a
network-agnostic API. On `localnet` it uses the nodeless mock; on `testnet`/`mainnet`
it uses the real `walrus-sdk` (the real backend is the remaining milestone — see
[Status](#status)).

> This is the end-user/dev companion to the working implementation plan
> `docs/dev/LOCALNET_WALRUS_PLAN.md`, which holds the Gate-0 proof, line-level
> integration map, and risk analysis.

## Architecture

```
   WalrusStore::for_workdir("localnet")     WalrusStore::for_workdir("testnet")
                  |                                        |
            localnet/mock feature                   real backend (walrus-sdk)
                  |                                        |
         LocalnetMockStore                          RealWalrusStore   (walrus-sdk)
                  |
        +---------+----------+
        | suibase Helper      |  per-store instance; select_workdir("localnet") for
        | (rust/helper)       |  keystore / address / config-dir discovery
        +---------+----------+
                  |
   PTBs on localnet Sui (localhost:9000):
     reserve_space -> register_blob(s) -> certify_blob(s)   <- off-node held BLS key
     create_storage_pool -> register_pooled -> certify_pooled
                  |
   blob bytes on the local filesystem (content-addressed by blob_id)
```

Two layers:

- **Layer A (bash, M1):** on `localnet start`/`regen`, when the feature is enabled,
  `deploy_walrus_localnet()` publishes the Walrus Move packages into the freshly
  regenerated localnet, stakes an N=1 committee off-node, creates + funds a SUI→WAL
  exchange, and writes a descriptor (`config/walrus-localnet.yaml`) with the object ids
  + the held committee BLS key. Idempotent (keyed on the live chain id).
- **Layer B (Rust, M2/M3):** the `walrus-local-sdk` crate. Its `LocalnetMockStore`
  engine reads that descriptor + the workdir wallet, builds the PTBs, signs certificates
  off-node, and serves bytes from disk. On top of the engine sits `WalrusLocalClient` —
  a **drop-in mirror of the Mysten Walrus SDK** (`walrus_sdk`) that returns the SDK's own
  types, so caller code is identical on localnet and on a real network (the caller picks
  the crate by network; see [Using it from Rust](#using-it-from-rust)).

## Enabling it

Nodeless Walrus is **off by default** — a plain localnet is unchanged. (The crate itself
is localnet-only; only the localnet-tools bins / explicit tests pull the heavy graph.)
To turn it on:

```yaml
# ~/suibase/workdirs/localnet/suibase.yaml
walrus_local_enabled: true
```

Then regenerate the localnet so the deploy hook runs:

```bash
~/suibase/scripts/localnet regen
```

On success the descriptor `~/suibase/workdirs/localnet/config/walrus-localnet.yaml`
exists, and `WalrusLocalClient::for_workdir("localnet")` works.

If `walrus_local_enabled: true` but you have **not** regenerated yet (the descriptor is
missing, or its `chain_id` no longer matches the live localnet), `localnet start`
and `localnet status` print a non-fatal footer advising a `localnet regen` — the
localnet Sui node still runs, but the Walrus contracts are not deployed until the
regen hook runs. The deploy binary
(`walrus-localnet-deploy`) is **designed to ship precompiled** via
`chainmovers/sui-binaries` (the consuming side is wired in `consts.yaml` /
`__walrus-localnet-deploy.sh`); until that build pipeline is live, a dev checkout builds
it from `rust/localnet-tools` and stages it under `workdirs/common/bin/`.

## Why nodeless certify works {#why-nodeless-certify-works}

`certify_blob` verifies a BLS-aggregate signature against the on-chain committee — no
networking, no node liveness. Two facts make a single off-node signature a valid quorum:

- **Trivial quorum for N=1.** The verifier requires `3·weight ≥ 2·n_shards + 1`. A
  one-member committee holds all shards (`weight == n_shards`), so an all-signers bitmap
  satisfies it for every `n_shards ≥ 1`.
- **`blob_id` is caller-chosen, not bound to real slivers.** Registration only enforces
  `derive_blob_id(root_hash, encoding_type, size) == blob_id`, where `root_hash` is an
  opaque caller value. The chain never runs erasure coding. So whoever holds the
  committee key can pick a `blob_id`, register, and self-certify.

The signing reuses `walrus-core` verbatim (`Confirmation::new` +
`ProtocolKeyPair::sign_message`; fastcrypto BLS12381 min_pk, matching the on-chain
`bls12381_min_pk_verify`) — no hand-rolled BCS. The certify message's epoch must equal
the **current committee epoch** at submission, so the client re-reads the epoch
immediately before signing.

## Using it from Rust {#using-it-from-rust}

`walrus-local-sdk` is a **drop-in mirror of the Mysten Walrus SDK** (`walrus_sdk`):
`WalrusLocalClient` mirrors the method signatures of
`walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` and returns the SDK's
**own** types (`BlobStoreResult`, `QuiltStoreResult`, `ClientResult`, …). The caller
dispatches by network — on a real network you use `walrus_sdk` directly; on localnet you
use this crate — and the call sequence is byte-for-byte identical:

```rust
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;
use walrus_core::encoding::Primary;

let client = WalrusLocalClient::for_workdir("localnet").await?;   // nodeless mock
let args = StoreArgs::default_with_epochs(5);

let results = client.reserve_and_store_blobs(vec![b"hello".to_vec()], &args).await?;
let blob_id = results[0].blob_id().unwrap();
let bytes = client.read_blob::<Primary>(&blob_id).await?;          // served from fs
let removed = client.delete_owned_blob(&blob_id).await?;          // burn + remove bytes

// Quilts (V1) via the sub-client, mirroring walrus_sdk's QuiltClient:
let qc = client.quilt_client();
let quilt = qc.construct_quilt(&blobs, walrus_core::EncodingType::RS2).await?;
let qres = qc.reserve_and_store_quilt(quilt, &args).await?;       // QuiltStoreResult
```

**Design intent (do not regress).** The real path is `walrus_sdk` used *directly* — this
crate inserts no glue there, so a bug here can only ever affect localnet (devs). The one
real-facing seam, `compat::WalrusApi` (a tiny generic trait the parity tests use to run
ONE body against both backends), has a *pure-forwarding* impl for `WalrusNodeClient`.

**Lower-level engine + pools.** `client.engine()` (a `LocalnetMockStore`) is the
localnet-only engine the mirror wraps. It also offers storage pools — not part of the
`walrus_sdk` high-level surface, so they live on the engine:

```rust
let store = client.engine();
let cap = store.encoded_size(10_000).await? * 4;
let pool = store.create_pool(cap, 10).await?;
let p = store.store_pooled(&pool.pool_id, b"pooled payload").await?;
store.delete_pooled(&pool.pool_id, &p.blob_id).await?;
```

Notes:

- `blob_id` is the canonical Walrus `BlobId` (URL-safe base64). Identical content yields
  the same `blob_id`; a re-store dedups on it, returning `AlreadyCertified` while the
  on-chain blob stays certified + unexpired.
- Bytes are content-addressed and shared on disk; the `blob_id → on-chain-object` index
  is scoped (standalone vs per-pool), so identical content can be both standalone and
  pooled (in one or more pools) without aliasing. Shared bytes are removed only when the
  last reference is deleted.
- Pool capacities are in **encoded** bytes (use `encoded_size()` to size a pool).
- `store_pooled` is **not** content-idempotent within a pool: re-storing identical bytes
  into the same pool aborts (the pool's blob table rejects the duplicate `blob_id`).

## HTTP facade — `sb-local` (the Walrus aggregator/publisher wire API)

`sb-local` ("suibase localnet") is a **standalone, long-running, localnet-only HTTP
server** that exposes the **Walrus aggregator + publisher wire API**, backed by the same
`LocalnetMockStore`. It is a **drop-in replacement for the real `walrus daemon`** (the
combined aggregator + publisher): point any existing Walrus HTTP client — `curl`/`fetch`,
walrus-sites, anything — at sb-local by **changing only the URL**. It is the front door
for polyglot clients; the Rust `WalrusLocalClient` API above is the other front door to
the **same** store (same filesystem dir, same `blob_id` derivation), so a blob written
via HTTP is readable via Rust and vice-versa, with no coordination.

It is managed exactly like the localnet faucet — started on `localnet start` and stopped
on `localnet stop`, **gated on `walrus_local_enabled=true`** — with its own independent bind/port
(`sb_local_host_ip`/`sb_local_walrus_port`, default `localhost:45840`). **The suibase-daemon is NOT
involved.** `localnet status` shows a `Walrus API` line. It is a glibc binary shipped in
the `localnet-tools` asset alongside `walrus-localnet-deploy` (source-built on dev).

Routes (one process, one port, one router — the real `daemon` topology):

| Method | Path | Returns |
|---|---|---|
| `GET` | `/status` | `OK` (liveness) |
| `GET` | `/v1/blobs/{blob_id}` | raw bytes + `ETag`/`Cache-Control`/`X-Content-Type-Options`; HTTP `Range` → `206`; `404` if absent |
| `GET` | `/v1/blobs/by-object-id/{object_id}` | raw bytes (resolve by Sui object id) |
| `PUT` | `/v1/blobs` | `200` `BlobStoreResult` (camelCase tagged enum: `newlyCreated` / `alreadyCertified`). Query: `epochs` (default 1), `permanent`/`deletable` (no-op), `send_object_to=<addr>` xor `share=true` |
| `PUT` | `/v1/quilts` | `200` `QuiltStoreResult` (multipart: each file field-name = patch identifier; optional `_metadata` JSON `[{identifier,tags}]`) |
| `GET` | `/v1/blobs/by-quilt-patch-id/{id}` | patch bytes + `X-Quilt-Patch-Identifier` |
| `GET` | `/v1/blobs/by-quilt-id/{quilt_id}/{identifier}` | patch bytes |
| `GET` | `/v1/quilts/{quilt_id}/patches` | `[{identifier, patchId, tags}]` |

```bash
# Store + read a blob over HTTP (drop-in: same calls work against testnet/mainnet
# aggregators/publishers — only the URL changes).
BASE=http://localhost:45840
ID=$(curl -s -X PUT --data-binary @file "$BASE/v1/blobs?epochs=3" | jq -r '.newlyCreated.blobObject.blobId')
curl -s "$BASE/v1/blobs/$ID" -o out          # bytes == file
```

**Real (cross-environment) blob ids.** Because the engine derives `blob_id` with
walrus-core's REAL encoder (M0), a localnet id is **bit-identical** to what
testnet/mainnet mint for the same bytes (verified: equals `walrus blob-id --n-shards 1000`).
A client can compute/verify ids and carry blob identity across networks.

**Out of scope (by design):** the upload-**relay** protocol (it needs real storage nodes —
nodeless can't provide it), `/v1alpha` streaming/concat, and JWT auth. The node-talking
`@mysten/walrus` SDK targets testnet/mainnet storage nodes, **not** nodeless localnet —
localnet clients use this HTTP wire API (or the Rust `WalrusLocalClient` API) instead. The
pool ops aren't in the Walrus HTTP spec and stay Rust-engine-only.

## WAL funding

`reserve_space` does not auto-convert SUI→WAL. On the first paying op per process the
mock swaps a fixed amount of SUI for WAL via the descriptor's exchange object (minted +
funded by the deploy). This is faucet-cheap on a regen-able localnet.

## Why a localnet-only mirror (and not "one crate, both networks")

`walrus-local-sdk` is **localnet-only on purpose**. A real network uses `walrus_sdk`
**directly** — the enclave / any production caller links the Mysten crate, not this one —
so there is no shared abstraction that a bug here could break on a real network. This crate
freely pulls the heavy walrus/Sui graph (incl. `suibase`, RocksDB) because it is never
enclave-linked. Drop-in parity is achieved by *mirroring the SDK's signatures + types*,
verified by the parity tests, **not** by a wrapper that both paths share.

## Testing

- **Unit (pure logic, no live localnet):** `cargo test --lib` (real blob-id ==
  walrus-core encoder, quilt pack/unpack + patch-id round-trip, blob-id parse, descriptor
  null-normalization, direct-rpc wallet rewrite, fs key).
- **Live integration (gated on `WALRUS_LOCALNET_TEST=1`, needs a running localnet with
  the descriptor present):**
  - `tests/localnet_roundtrip.rs` — the backend-agnostic parity body (store → read → dedup
    → delete via `compat::WalrusApi`) through `WalrusLocalClient`, plus a quilt round-trip.
  - `tests/localnet_pool.rs` — pool create → store_pooled → status → extend → grow → delete
    (on the engine).
  - `tests/localnet_pool_namespace.rs` — identical content standalone + pooled coexist.
- **Real-network parity (on-demand, fund-gated):** `tests/testnet_parity.rs` runs the
  **same** generic `parity_roundtrip` body against a real `walrus_sdk::WalrusNodeClient`.
  Set `WALRUS_TESTNET_TEST=1` (+ `WALRUS_TESTNET_CONFIG`, else the walrus CLI default) and
  a funded wallet (SUI for gas, WAL for storage). Skips cleanly without config; fails loudly
  without funds. This is the drop-in proof: one body, both backends, SDK types throughout.
- **Live HTTP (sb-local), via curl — `scripts/tests/050_walrus_tests/test_sb_local_http.sh`:**
  PUT/GET round-trip, real-id equality vs `walrus blob-id`, `Range`→206, dedup, 404,
  Rust/HTTP interop, and the full quilt round-trip. Self-skips when sb-local is not
  reachable (safe in the fast suite); set `SB_LOCAL_HTTP_TEST=1` to make a skip a failure.
- **CI:**
  - `walrus-localnet-integration.yml` — heavy on-demand/weekly: builds the library + deploy
    bin + sb-local, deploys nodeless Walrus on a real localnet via the regen hook, runs the
    unit tests + all three live suites + the sb-local HTTP wire test.
  - `staging.yml` `validate-localnet-tools` — validates the PRECOMPILED `walrus-localnet-deploy`
    (deploy on a real localnet) and the PRECOMPILED `sb-local` (HTTP wire round-trip).

```bash
# Local run (needs: localnet started, walrus_local_enabled, regen'd so the deploy ran)
cd rust/walrus-local-sdk
WALRUS_LOCALNET_TEST=1 cargo test --test localnet_roundtrip
```

## Status

| Milestone | State |
|---|---|
| M0 Gate-0 spike (off-node certify proof) | ✅ done |
| M1 Nodeless deploy (Layer A bash) | ✅ done |
| M2 mock engine store/read/stat/extend/delete | ✅ done |
| M3 Pool ops (create/store_pooled/delete_pooled/status/extend/grow) | ✅ done |
| Real cross-environment blob ids (walrus-core encoder) | ✅ done (localnet id == `walrus blob-id`) |
| `sb-local` HTTP facade (aggregator + publisher + quilt) | ✅ done (live curl round-trip verified) |
| `sb-local` localnet lifecycle (start/stop/status) | ✅ done |
| `WalrusLocalClient` — drop-in `walrus_sdk` mirror (SDK types + `ClientResult`) | ✅ done (live localnet round-trip + quilt) |
| Real-network drop-in parity test (`testnet_parity.rs`, fund-gated) | ✅ harness done (on-demand) |

## References

- Working plan + Gate-0 proof + risks: `docs/dev/LOCALNET_WALRUS_PLAN.md`
- sb-local HTTP facade plan: `docs/dev/SB_LOCAL_PLAN.md`
- Library crate: `rust/walrus-local-sdk/` (mock engine `src/localnet.rs`, the SDK mirror
  `src/lib.rs` = `WalrusLocalClient`, the dispatch seam `src/compat.rs`; no binaries)
- Bins crate: `rust/localnet-tools/` (deploy bin `src/bin/walrus_localnet_deploy.rs`,
  HTTP server `src/bin/sb_local/`, embedded contracts) — builds on `walrus-local-sdk`
- Deploy orchestration: `scripts/common/__walrus-localnet-deploy.sh`
- sb-local lifecycle: `scripts/common/__sb-local-process.sh`
- Sibling style reference: `docs/dev/WALRUS_RELAY_FEATURE.md`
