# Local Walrus on Suibase Localnet — Implementation Plan

**Status:** In progress on `dev` · **Gate-0 verdict:** ✅ **PASS — GO**
(empirically confirmed on suibase localnet, 2026-06-17).

**Milestone progress:**
- ✅ **M0** Gate-0 spike — PASS (off-node certify verifies, extend works, regen-survives).
- ✅ **M1** Localnet deploy (Layer A) — `walrus_local_enabled` flag (default **off**, no auto-deploy),
  `walrus-localnet-deploy` bin (embeds vendored contracts), regen hook, static config test.
- ✅ **M2** Localnet store/read/stat/extend/delete — content dedup, write-bytes-before-certify,
  off-node held-key certify (N=1 committee). Unit tests + gated live round-trip + heavy CI workflow.
- ✅ **M3** Storage pools — create/store_pooled/delete_pooled/pool_status/extend/grow + `encoded_size()`.
  Off-node held-key certify for **pooled** (Deletable) blobs verified live. 3-lens adversarial
  review folded in (epoch-at-certify, pool-scoped sidecars, non-idempotency doc). Pool + namespace
  regression tests (red-checked).
- ✅ **M4** Real-network usage = `walrus_sdk` **directly** (no Suibase glue) + the **drop-in mirror**
  for localnet. The mirror returns the SDK's own types so caller code is identical across networks;
  `compat::WalrusApi` is the dispatch seam. **Live testnet drop-in parity verified** (see below).
- ✅ **M5** Quilts (generic over `QuiltVersion`), byte-range read, and the `sb-local` HTTP facade
  (aggregator + publisher) + Suibase wiring (regen hook, `localnet status` line, scripts test).

> This is the **working implementation plan / record** for a Suibase feature: a self-contained local
> Walrus on the suibase localnet, plus a localnet-only Rust client that is a **drop-in mirror** of
> the Mysten Labs Walrus SDK (`walrus_sdk`). It mirrors the structure of
> `docs/dev/WALRUS_RELAY_FEATURE.md`.
>
> Product context and the original pinned decisions live in a private planning brief (Suiftly), which
> is intentionally **not** copied into this repo; this plan is generic Sui+Walrus infrastructure with
> no downstream-product content.

## The mirroring model (read this first)

The crate `rust/walrus-local-sdk` (LOCALNET-ONLY) is a **drop-in mirror** of `walrus_sdk`:

- On a **real network** (testnet/mainnet) you use `walrus_sdk` **directly** — this crate inserts no
  glue on that path.
- On **localnet** you use `WalrusLocalClient` from this crate.
- Caller code is **identical** across networks: the mirror exposes the same method signatures and
  returns the SDK's **own** types (`BlobStoreResult`, `QuiltStoreResult`, `ReadByteRangeResult`,
  `ClientResult`, …). Drop-in parity is achieved by **mirroring** the SDK and proving it with parity
  tests — **not** by a shared wrapper that both paths route through.

Because nothing wraps the real path, a bug in this crate can only ever affect localnet (devs); the
real network is never touched. The localnet burden lives entirely in the localnet mock engine
(`LocalnetMockStore`) and the thin reshaping in the mirror; the one real-facing seam
(`impl compat::WalrusApi for WalrusNodeClient`) is **pure forwarding** (one SDK call per method, no
logic) precisely to keep that risk at zero.

This crate is never linked enclave-side, so it freely pulls the heavy walrus/Sui graph (incl.
`suibase`). It has **no cargo features**: it always pulls the full localnet graph.

## Overview

Two coupled capabilities:

1. **Self-contained local Walrus** on a Suibase localnet: real Walrus `Blob`/`Storage`/`StoragePool`
   objects created via Sui PTBs against genuinely-published Walrus Move packages, with blob
   **bytes stored on the local filesystem** (keyed by `blob_id`). **No storage nodes, no RocksDB,
   no erasure-coded slivers retained.** The single operation that normally requires a live node —
   `certify_blob` — is satisfied **off-node** by holding the committee's BLS secret key and
   self-signing the certificate. The blob id + Merkle root are still computed by walrus-core's
   **real** encoder, so a localnet blob id is bit-identical to what testnet/mainnet mint for the
   same content (cross-environment blob identity).

2. **`WalrusLocalClient`** — the localnet drop-in mirror of `walrus_sdk` (see above). Construct via
   `WalrusLocalClient::for_workdir("localnet")` (only `"localnet"` is valid; any other name is an
   error telling the caller to use `walrus_sdk` directly). The caller names the network explicitly;
   there is **no global "active" workdir** consulted (see [Network selection](#network-selection)).

### Why off-node certify is possible (the crux — independently source-verified)

`certify_blob` is a **pure BLS-aggregate signature check** against the on-chain committee — no node
liveness or networking is involved
(`contracts/walrus/sources/system.move:180` → `system_state_inner.move:351` → `bls_aggregate.move:202`).
Two facts make a single off-node signature sufficient:

- **Quorum is trivial for N=1.** The verifier requires `3·weight ≥ 2·n_shards + 1`
  (`bls_aggregate.move:164`). A one-member committee holds all shards (`weight == n_shards`), so an
  all-signers bitmap yields `aggregate_weight == n_shards` and `3·n_shards ≥ 2·n_shards + 1` holds
  for every `n_shards ≥ 1`. The reconstructed aggregate key trivially equals the committee total.
- **`blob_id` is caller-controlled, not bound to real sliver data.** Registration only enforces the
  tautology `derive_blob_id(root_hash, encoding_type, size) == blob_id`, where `root_hash` is an
  opaque caller-supplied `u256` (`blob.move:119-193`). The chain never runs erasure coding and never
  contacts a node. So a deployer who holds the committee secret key can pick any `blob_id`, register,
  and self-certify. (The engine still computes the *real* `root_hash`/`blob_id` via walrus-core so
  the id matches real networks — it just retains no slivers.)

The signing path reuses `walrus-core` verbatim (no hand-rolled BCS): build a `Confirmation` and sign
with `ProtocolKeyPair::sign_message` (`keys.rs:261-273`); the scheme is fastcrypto BLS12381 **min_pk**,
matching the on-chain native `bls12381_min_pk_verify`.

## Gate-0 Results (empirical — PASS) {#gate-0-results}

Executed 2026-06-17 against a live suibase localnet (Sui `1.73.1`, the version the pinned Walrus
reference targets) using binaries built from walrus rev `1049b56` (walrus 1.51.0 — what real
networks run). Build dependency resolved without sudo: `libclang.so` from the `libclang` pip wheel +
GCC freestanding headers via `BINDGEN_EXTRA_CLANG_ARGS`. Two independent proofs:

1. **Walrus's own `test_register_certify_blob` passed** (`cargo test -p walrus-sui --features test-utils -- --ignored`):
   `2 passed; 0 failed`. This test performs off-node held-key `certify_blob` with **no storage nodes**
   (`TestNodeKeys::blob_certificate_for_signers` → `certify_blobs`), confirming the fastcrypto min_pk
   DST matches Sui's native `bls12381_min_pk_verify` — the #1 risk, **resolved**.
2. **A throwaway spike on suibase localnet passed end-to-end** (`crates/walrus-sui/examples/localnet_nodeless_certify.rs`):
   publish Walrus contracts → set up + stake an N=1 committee off-node → `reserve_space` →
   `register_blob` (uncertified) → **off-node held-key `certify_blob` OK** → `extend_blob` OK (proves
   the cert is real) → filesystem byte round-trip OK. Re-confirmed after `localnet regen`.

Separately, `walrus-deploy deploy-system-contract` published `wal`/`wal_exchange`/`walrus`/
`walrus_subsidies` with an N=1 deterministic committee and emitted all object IDs + the held committee
BLS key (`testbed_config.yaml: nodes[0].keypair`) — **no storage nodes** — validating the Layer-A
deploy mechanism.

**Two empirical findings that refined the plan (both mechanical, neither a blocker):**

- **Committee staking is a real step `walrus-deploy` does not perform.** `deploy-system-contract` and
  `generate-dry-run-configs` leave the committee **empty** (`members: []`); the node is only registered
  + staked by `register_committee_and_stake` + `end_epoch_zero` (`walrus-sui` test-utils). Layer A (M1)
  performs this step itself.
- **Localnet has read-after-write lag.** After `initiate_epoch_change`, the fullnode (`:9000`) takes a
  few seconds to reflect the new epoch/committee. The deploy bin and `LocalnetMockStore::open` both
  poll `current_committee()` / retry the connect rather than reading once.

## Gate-0 Spike (throwaway — must PASS before building the feature)

**Hard GO/NO-GO:** if an off-node held-key signature will not pass `certify_blob`, **STOP**.

**Signed-byte layout** — the 40 bytes the BLS signature covers, for a Permanent blob, produced by
`bcs::to_bytes(&walrus_core::messages::Confirmation::new(epoch, blob_id, BlobPersistenceType::Permanent))`:

| offset | len | value |
|---|---|---|
| 0 | 1 | `0x01` intent type `BLOB_CERT_MSG` |
| 1 | 1 | `0x00` intent version |
| 2 | 1 | `0x03` app id `STORAGE` |
| 3 | 4 | `epoch` u32 little-endian (**must equal the live system epoch**) |
| 7 | 32 | `blob_id` raw bytes (peeled on-chain as a `u256`, LE) |
| 39 | 1 | `0x00` Permanent |

**Scheme:** `ProtocolKeyPair = TaggedKeyPair<fastcrypto::bls12381::min_pk::BLS12381KeyPair>` —
pubkeys compressed G1 (48 B), signatures compressed G2 (96 B), IETF DST baked into fastcrypto/blst
(the sign→verify round trip is the proof). **Do not hand-roll BCS** — use `Confirmation::new` +
`ProtocolKeyPair::sign_message`.

**Recipe (PASS = all three checks below):**

1. **Build:** `cargo build --release -p walrus-service --bin walrus-deploy --features deploy`
   (the `deploy` feature pulls `walrus-sui/test-utils`) and `--bin walrus`. Build dependency: the
   whole Sui/Walrus crate graph needs RocksDB → `zstd-sys`/`bindgen` → **libclang**.
2. **Localnet:** `~/suibase/scripts/localnet start`. Faucet `http://127.0.0.1:9123/gas`; fullnode RPC
   **`http://localhost:9000`**.
3. **Deploy contract (N=1, deterministic):**
   `walrus-deploy deploy-system-contract --working-dir ./wd --sui-network localnet --contract-dir ./contracts --n-shards 1 --host-addresses 127.0.0.1 --deterministic-keys --with-wal-exchange`.
   Capture `package_id / system_object / staking_object / exchange_object` and `nodes[0].keypair`.
4. **Stake + end epoch 0 (the step `deploy-system-contract` omits):**
   `walrus-deploy generate-dry-run-configs --working-dir ./wd` → runs `register_committee_and_stake`
   + `end_epoch_zero`. Confirm the epoch advanced past 0 and the single node is the live committee.
5. **Fund (real WAL):** `walrus generate-sui-wallet --sui-network localnet --use-faucet`, then
   `walrus get-wal --exchange-id <exchange_object> --amount <mist>` (1:1 default rate).
6. **Spike bin** (throwaway): reserve → register (Permanent, `blob_id = derive_blob_id(...)`) → sign
   off-node (`ProtocolKeyPair::from_str` + `Confirmation::new` + `sign_message`; aggregate one
   signature, `bitmap = [0x01]`) → `certify_blob` (**PASS #1** = `BlobCertified`) → `extend_blob`
   (**PASS #2** = succeeds, since extend hard-requires `assert_certified_not_expired`) → fs round-trip.
7. **Regen survival:** `~/suibase/scripts/localnet regen`, re-run steps 3–6. **PASS #3** = the full
   flow reproduces deterministically.

Any `ESigVerification` on certify = **HARD NO-GO**.

## Architecture

```
  real network (testnet/mainnet)            localnet
  ────────────────────────────              ────────
  walrus_sdk::node_client::WalrusNodeClient   ◄ mirror ►  WalrusLocalClient::for_workdir("localnet")
  (used DIRECTLY, no glue)                          │
          │                                   LocalnetMockStore  (the localnet engine)
          │                                         │
          │                                +--------+--------+
          │                                | suibase Helper  |  select_workdir("localnet")
          │                                | (discovery)     |  keystore + workdir paths
          │                                +--------+--------+
          │                                         │
          │                       PTBs: reserve_space -> register_blob ->
          │                              certify_blob (held BLS key, off-node sign) -> extend_blob
          │                                         │
          ▼                                  bytes on filesystem (keyed by blob_id)

      compat::WalrusApi   ── one generic body runs against either side (parity tests)
```

`WalrusLocalClient` mirrors `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` and
returns the SDK's own types. It WRAPS the engine and reshapes engine results into the SDK wire types
(e.g. `StoredBlob` → `BlobStoreResult::NewlyCreated` / `::AlreadyCertified`). The engine is reachable
via `client.engine()` for the localnet-only lower-level surface (storage pools, rich `StoredBlob`,
quilt index) that is not part of `walrus_sdk`'s high-level API.

### Network selection {#network-selection}

The caller picks the network **explicitly by name** — there is **no `for_active_workdir()` and no read
of the global "active" workdir**. (The suibase global active workdir is a contended, machine-global
symlink that multiple processes fight to switch; do not build new dependencies on it.)

- `WalrusLocalClient::for_workdir(name)` accepts only `"localnet"`; any other name returns an error
  directing the caller to `walrus_sdk` directly. (`open()` is the equivalent no-arg constructor.)
- The engine owns its **own `suibase::Helper` instance** and calls `helper.select_workdir("localnet")`
  with the **explicit name** — never the special string `"active"`. `select_workdir` is instance-local
  (it does not touch the global symlink), so **multiple clients coexist in one process** with no
  contention. **No suibase code changes are required** for this.

### Layers

- **Layer A (bash + the `walrus-localnet-deploy` bin):** publishes Walrus into the freshly-regenerated
  localnet (embedded vendored contracts), stakes the N=1 committee, ends epoch 0, and writes the
  descriptor `<workdir>/config/walrus-localnet.yaml` (package id + system/staking/treasury/exchange
  object ids + held committee BLS keypair).
- **Layer B (Rust):** the localnet engine `LocalnetMockStore` builds PTBs and signs certs off-node;
  `WalrusLocalClient` mirrors `walrus_sdk` over it; `sb-local` exposes it over HTTP.

## Client surface (the real current API)

Caller code is the same on localnet and a real network — only the constructor differs
(`WalrusLocalClient::for_workdir("localnet")` vs `walrus_sdk`'s `WalrusNodeClient`):

```rust
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;
use walrus_core::encoding::Primary;

let client = WalrusLocalClient::for_workdir("localnet").await?;   // localnet mock
let args = StoreArgs::default_with_epochs(5);

// store -> Vec<BlobStoreResult> (the SDK's own type)
let results = client.reserve_and_store_blobs(vec![b"hello".to_vec()], &args).await?;
let blob_id = results[0].blob_id().unwrap();

// whole-blob read (generic axis; ignored on localnet — bytes are whole on disk)
let bytes = client.read_blob::<Primary>(&blob_id).await?;
let bytes = client.read_blob_primary(&blob_id).await?;   // no-turbofish convenience

// delete (idempotent; returns the count removed)
let removed = client.delete_owned_blob(&blob_id).await?;
```

Core blob methods (mirror `WalrusNodeClient`): `reserve_and_store_blobs`, `read_blob::<U>` /
`read_blob_primary`, `delete_owned_blob`, `get_blob_by_object_id`.

### Byte-range read (critical for performance)

`byte_range_read_client()` returns a sub-client mirroring `walrus_sdk`'s `ByteRangeReadClient`. It
fetches a **slice** of a large blob without pulling the whole blob:

```rust
let r = client
    .byte_range_read_client()
    .read_byte_range(&blob_id, /*start*/ 1024, /*length*/ 4096)
    .await?;                              // -> ReadByteRangeResult { data, unencoded_blob_size }
```

The mirror replicates the SDK's input validation **exactly**, including the error kinds and messages
(`ClientErrorKind::ByteRangeReadInputError` for zero length, overflow, out-of-bounds), validated
before the blob is touched (SDK order). On localnet the bytes are whole on disk, so the range is a
plain slice. **Tested:** exhaustive pure unit tests (validate + slice), an extensive live localnet
integration test (`tests/localnet_byte_range.rs`: a 16 KB blob, many ranges + all error cases), and
the real-testnet parity test.

### Quilts (generic over `QuiltVersion`)

`quilt_client()` returns a sub-client mirroring `walrus_sdk`'s `QuiltClient`. The construct/store
methods are **generic over `V: QuiltVersion`**, structurally mirroring the SDK (dispatch through
`V::QuiltConfig::get_encoder`; iterate `quilt_index().patches()`):

```rust
use walrus_core::encoding::quilt_encoding::{QuiltStoreBlob, QuiltVersionV1};
use walrus_core::EncodingType;

let qc = client.quilt_client();
let blobs = vec![QuiltStoreBlob::new_owned(b"a".to_vec(), "patch-a".into())?];
let quilt = qc.construct_quilt::<QuiltVersionV1>(&blobs, EncodingType::RS2).await?;   // pure compute
let result = qc.reserve_and_store_quilt::<QuiltVersionV1>(quilt, &args).await?;        // -> QuiltStoreResult
```

Reads: `get_blobs_by_identifiers`, `get_blobs_by_ids` (by public `QuiltPatchId`), `get_all_blobs`. A
quilt packs many named blobs into ONE blob + an embedded index (100% client-side pure compute, no
storage nodes); the packed bytes go through the engine's normal `store_blob` path, so the resulting
blob id IS the real quilt id and the quilt blob dedups/extends/deletes like any other Permanent blob.
Live-verified on localnet.

### Storage pools (engine-only — not part of `walrus_sdk`'s high-level surface)

Pools are a localnet lower-level capability on the engine, reached via `client.engine()`:
`encoded_size`, `create_pool`, `store_pooled`, `delete_pooled`, `pool_status`, `extend_pool`,
`grow_pool`. A pool reserves a chunk of **encoded** capacity that many pooled blobs share for the
pool's lifetime; pooled blobs are registered Deletable (so the held-key certify message binds to the
pooled blob's own object id) and certified off-node, same as the standalone path. Bytes are
content-addressed and shared on disk; sidecars are scoped per-pool so identical content can be pooled
in several pools (and/or stored standalone) without aliasing.

### `compat::WalrusApi` — the dispatch / parity seam

`compat::WalrusApi` is a small generic trait over the **non-generic blob core**
(`reserve_and_store_blobs` / `read_blob_primary` / `delete_owned_blob` / `read_byte_range`). It lets
ONE generic body run against either backend. Two impls:

- for `WalrusLocalClient` (localnet): delegates to the inherent mirror methods;
- for `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` (real): **pure forwarding** —
  exactly one SDK call per method, no logic, so the real path stays transparent and carries zero
  shared-bug risk.

The generic reads (`read_blob::<U>`), the quilt sub-client, and introspection stay as inherent
methods on each client (their type generics / borrowed sub-client types resist a single object-safe
trait, and forcing them in would add glue = risk for little gain).

## `sb-local` HTTP facade

`sb-local` (a bin in `rust/localnet-tools`, built on the engine) serves a wire-faithful subset of the
real Walrus aggregator + publisher HTTP API from one process / one port / one router:

- `GET  /v1/blobs/{blob_id}` — aggregator read (raw bytes; honors a `Range:` request header)
- `GET  /v1/blobs/by-object-id/{object_id}` — aggregator read by Sui `Blob` object id
- `PUT  /v1/blobs` — publisher store (returns a wire `BlobStoreResult`)
- `PUT  /v1/quilts` — publisher quilt store
- `GET  /v1/blobs/by-quilt-patch-id/{id}` — quilt patch read by public id
- `GET  /v1/blobs/by-quilt-id/{quilt_id}/{ident}` — quilt patch read by identifier
- `GET  /v1/quilts/{quilt_id}/patches` — list quilt patches
- `GET  /status` — liveness

It is auto-started by `localnet regen` when the feature is enabled, and shows up on the
`localnet status` line.

## Suibase integration points

| # | File | Change |
|---|---|---|
| 1 | `scripts/common/__globals.sh` `is_walrus_supported_by_workdir()` | add a `localnet` arm, gated behind the `walrus_local_enabled` flag. |
| 2 | `scripts/common/__globals.sh` walrus config/rpc repair | add a localnet arm; **skip** the static per-field repair (IDs come from deploy, not the template); localnet rpc → `http://localhost:9000`. Defensive (`return 0` on missing files). |
| 3 | `scripts/common/__walrus-localnet-deploy.sh` | run the `walrus-localnet-deploy` bin: deploy + stake + end-epoch-0, write `<workdir>/config/walrus-localnet.yaml`. Idempotent; **no node/process management.** |
| 4 | `scripts/common/__workdir-exec.sh` regen flow | after the Sui wipe and before `start_all_services`: run the deploy + repair + (if enabled) start `sb-local`, **non-fatal** (`warn_user` on failure). |
| 5 | `scripts/defaults/localnet/suibase.yaml` | `walrus_local_enabled` flag (default off); non-colliding localnet ports for the `sb-local` HTTP server. |
| 6 | `rust/walrus-local-sdk` | the localnet-only drop-in mirror crate (`WalrusLocalClient` + the quilt / byte-range sub-clients + `compat::WalrusApi`) over the localnet engine. |
| 7 | `rust/localnet-tools` | the bins crate: `walrus-localnet-deploy` (embeds vendored contracts) + `sb-local` (HTTP facade), both built on `walrus-local-sdk`. |
| 8 | `scripts/tests/050_walrus_tests/` | scripts tests: non-destructive config wiring (fast suite) + the `sb-local` HTTP wire round-trip (CI). |

Conventions (`CONTRIBUTING.md`): work on a `dev`-derived branch; `shellcheck`; `export -f`; UPPERCASE
globals; must pass `scripts-tests` + `rust-tests`.

## Crate layout & tests

```
rust/walrus-local-sdk/        # LOCALNET-ONLY drop-in mirror of walrus_sdk (no cargo features)
  src/lib.rs                  #   WalrusLocalClient + LocalQuiltClient + LocalByteRangeReadClient
  src/localnet.rs             #   LocalnetMockStore (the localnet engine) + storage pools + quilts
  src/compat.rs               #   WalrusApi dispatch trait (localnet impl + pure-forwarding real impl)
  tests/                      #   localnet_roundtrip, localnet_byte_range, localnet_pool,
                              #   localnet_pool_namespace, testnet_parity, common/ (shared parity body)
rust/localnet-tools/          # bins built on walrus-local-sdk
  src/bin/walrus_localnet_deploy.rs
  src/bin/sb_local/           #   the HTTP facade
```

**Tests:**

- `cargo test --lib` — pure-logic unit tests (blob-id / descriptor / wallet / byte-range
  validate+slice), no live localnet.
- `WALRUS_LOCALNET_TEST=1 cargo test --test <name>` — the live localnet suites (`localnet_roundtrip`,
  `localnet_byte_range`, `localnet_pool`, `localnet_pool_namespace`) against a deployed localnet.
- `WALRUS_TESTNET_TEST=1` + `WALRUS_TESTNET_CONFIG=/path/to/client_config.yaml` —
  the fund-gated real-network parity test (`testnet_parity`).
- CI: `.github/workflows/walrus-localnet-integration.yml` — builds the bins, deploys self-contained Walrus
  on a real localnet via the regen hook, runs the unit tests + the live localnet suites + the
  `sb-local` HTTP wire round-trip. Expensive (the walrus/Sui graph compile), so it runs on demand,
  weekly, and when the Walrus crates change on `dev` — not on the per-push fast suites.

### Real-network DROP-IN PARITY — verified LIVE

`tests/testnet_parity.rs` runs the **exact same generic body** (`common::parity_roundtrip`, written
purely against `compat::WalrusApi`) that the localnet round-trip runs — but against a **real**
`walrus_sdk::node_client::WalrusNodeClient` on **testnet**. Fund-gated and verified live:
store → read → byte-range → dedup → delete of a real blob all succeeded. If the mirror's
signatures/types ever drift from the SDK the test won't compile; if behavior drifts, the two backends
disagree.

## Risks

1. ~~fastcrypto min_pk DST not verbatim in-repo~~ — **RESOLVED** (Gate-0): the sign→verify round trip
   passed. Always sign via `ProtocolKeyPair::sign_message` so the DST is whatever fastcrypto uses.
2. **`deploy-system-contract` / `generate-dry-run-configs` do NOT stake the committee** → Layer A
   stakes (`register_committee_and_stake` + `end_epoch_zero`) and **polls `current_committee()`** for
   the new epoch (localnet read-after-write lag, ~seconds).
3. **min_pk / key-format footgun** → load via `ProtocolKeyPair::from_str` (handles the flag byte);
   never raw `from_bytes`.
4. **`cert_epoch` must equal the live epoch at submission** → read the epoch just before signing and
   submit promptly (the engine re-reads `current_epoch()` between register and certify).
5. **`encoded_size`** must cover the contract-computed encoded length or register aborts
   `EResourceSize` — the engine computes it from the real metadata.
6. **Deletable blobs** bind the blob object id into the signed persistence byte (unknown until
   register executes) → the standalone path uses Permanent; the pool path registers first, then signs
   with `blob_persistence_type()`.

## Reference commit pins

- Walrus crates (`walrus-sdk` / `walrus-sui` / `walrus-core`) are git-pinned to **one** rev,
  `1049b56b6fc3ca5eff9ac601ae5ff507ea772fa0` (`1049b56`, walrus **1.51.0**), so the localnet
  contracts (published from that rev's `contracts/`) match the SDK used here and the blob ids match
  what real networks mint. Keep this rev in sync with the Sui version Suibase localnet runs
  (`testnet-v1.73.1`).
