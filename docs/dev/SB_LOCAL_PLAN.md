# `sb-local` — Suibase Localnet Walrus HTTP engine

> The `sb-local` HTTP facade for nodeless localnet Walrus. Companion to
> `docs/dev/LOCALNET_WALRUS_PLAN.md` (the nodeless-localnet design) and
> `docs/dev/LOCALNET_WALRUS_FEATURE.md` (the localnet Walrus feature). Read those
> first.
>
> This doc began as an implementation plan; the work is **DONE and live**. It is
> kept as the design-of-record for `sb-local` and the crate it builds on,
> `walrus-local-sdk`.

## What this is

`sb-local` ("suibase localnet") is a **standalone, long-running localnet-only HTTP
server** that exposes the **Walrus aggregator/publisher wire API**:
`GET /v1/blobs/{blob_id}` (read), `PUT /v1/blobs` (store), the **byte-range** read,
and the **QUILT** API (`PUT /v1/quilts` + quilt reads). It gives polyglot clients
(`fetch`/`curl`/any HTTP, walrus-sites) a **switch-by-URL** store/read interface
against localnet, with **real on-chain Blob creation** on PUT.

It is managed by the suibase localnet lifecycle **exactly like the localnet faucet**
(start on `localnet start` when `walrus_local_enabled`, stop on `localnet stop`). It
is a **glibc** binary shipped in the `localnet-tools` asset (alongside
`walrus-localnet-deploy`). **The suibase-daemon is NOT involved** — no proxying,
no daemon dependency, so the daemon stays musl/lean.

`sb-local` builds on the **`LocalnetMockStore` engine** in the
**`walrus-local-sdk`** crate (`rust/walrus-local-sdk/src/localnet.rs`). Its handlers
hold a single `LocalnetMockStore` built once at startup (`LocalnetMockStore::open()`,
shared via `Arc`), so blobs written over HTTP are byte-identical and interoperable with
the Rust SDK-mirror API and vice-versa — same dir, same `blob_id` derivation, no
coordination. Two front doors, one store.

## The crate it builds on — `walrus-local-sdk` (mirroring model)

`rust/walrus-local-sdk` is **localnet-only** and is a **drop-in mirror of the Mysten
Labs Walrus SDK** (`walrus_sdk`). On a real network you use `walrus_sdk` **directly**;
on localnet you use this crate; **caller code is identical** — same method signatures,
the SDK's own return types (`BlobStoreResult`, `QuiltStoreResult`,
`ReadByteRangeResult`, `ClientResult`, …). Drop-in parity is achieved by **mirroring
the SDK** and is **verified by parity tests** — not by a shared wrapper.

### Public surface

- **`WalrusLocalClient`** (`src/lib.rs`) mirrors
  `walrus_sdk::node_client::WalrusNodeClient<SuiContractClient>` and returns the
  SDK's own types. Construct via `WalrusLocalClient::for_workdir("localnet")` (only
  `"localnet"` is valid — any other name errors, directing you to `walrus_sdk`).
  - `reserve_and_store_blobs(Vec<Vec<u8>>, &StoreArgs) -> Vec<BlobStoreResult>` — honors
    `StoreArgs.persistence` (Permanent **or** Deletable; Deletable certifies via the
    object-id-bound held-key path).
  - `read_blob::<U: EncodingAxis>(&BlobId) -> Vec<u8>` (+ `read_blob_primary` convenience);
    a missing blob returns `ClientErrorKind::BlobIdDoesNotExist`.
  - `delete_owned_blob(&BlobId) -> usize` — SDK semantics: deletes only **deletable** owned
    blobs (a no-op returning `0` for a Permanent blob).
  - `get_blob_by_object_id(&ObjectID) -> BlobWithAttribute`
  - `reserve_and_store_blobs_in_storage_pool(Vec<Vec<u8>>, ObjectID, &StoreArgs) -> Vec<PooledBlobStoreResult>`
  - `blob_status(&BlobId) -> BlobStatus` — synthesized from local state (the SDK's status
    methods are node-quorum reads; localnet blobs are always certified). `Nonexistent` for
    an absent or pooled-only blob.
  - `engine() -> &LocalnetMockStore` — exposes the lower-level engine (NOT part of the
    SDK surface) for callers that need the engine-only APIs (e.g. storage pools) without
    re-opening. `sb-local` and the pool tests open a `LocalnetMockStore` directly instead.

- **Sub-clients** (mirror the SDK's sub-clients):
  - `quilt_client() -> LocalQuiltClient`:
    `construct_quilt::<V: QuiltVersion>(&[QuiltStoreBlob], EncodingType) -> V::Quilt`
    and `reserve_and_store_quilt::<V: QuiltVersion>(V::Quilt, &StoreArgs) ->
    QuiltStoreResult`. These are **generic over `QuiltVersion`** and structurally
    mirror `walrus_sdk`'s `QuiltClient` (dispatch via `V::QuiltConfig::get_encoder`;
    iterate `quilt_index().patches()`). Also `construct_quilt_from_paths` /
    `reserve_and_store_quilt_from_paths` (reuse `walrus_sdk`'s own path helpers). Reads:
    `get_blobs_by_identifiers`, `get_blobs_by_ids`, `get_blobs_by_tag`, `get_all_blobs`,
    `get_quilt_metadata`.
  - `byte_range_read_client() -> LocalByteRangeReadClient`:
    `read_byte_range(&BlobId, start: u64, length: u64) -> ReadByteRangeResult`.
    Mirrors `walrus_sdk`'s `ByteRangeReadClient` **exactly**, including the
    input-validation error kinds + messages
    (`ClientErrorKind::ByteRangeReadInputError`). **Critical for performance:** fetch a
    slice of a large blob without pulling the whole blob.

- **`compat::WalrusApi`** (`src/compat.rs`): a small generic dispatch trait
  (`reserve_and_store_blobs` / `read_blob_primary` / `delete_owned_blob` /
  `read_byte_range`). It lets **one generic body** run against either backend — used by
  the parity tests. The impl for the real `WalrusNodeClient` is **pure forwarding** to
  `walrus_sdk` (exactly one SDK call per method, no logic): the real path stays
  transparent, so the bug burden lives only on localnet (dev-only).

### The engine — `LocalnetMockStore` (nodeless mock)

`src/localnet.rs`. The nodeless mock **engine** the mirror wraps. It:

- creates **real** `Blob`/`Storage` objects on the suibase localnet Sui via PTBs +
  **off-node held-key certify** (N=1 committee BLS key from the deploy-written
  descriptor `config/walrus-localnet.yaml`),
- serves bytes from the **filesystem** (no storage nodes),
- computes **real cross-environment blob ids** via walrus-core's encoder
  (`compute_metadata` — pure local compute, no slivers retained), so a localnet
  `blob_id` is **bit-identical** to what testnet/mainnet mint for the same content
  (and to `walrus blob-id --n-shards 1000`). Quilt ids are real for free.

**Storage pools** live on the engine (they are NOT part of `walrus_sdk`'s high-level
surface): `encoded_size` / `create_pool` / `store_pooled` / `delete_pooled` /
`pool_status` / `extend_pool` / `grow_pool`. The lower-level handle/metadata types
`BlobHandle` / `BlobMeta` / `PoolHandle` / `PoolStatus` belong to the engine layer.

**Shared blob directory (verified):**
- Bytes: `$HOME/suibase/workdirs/localnet/config/walrus-localnet-blobs/<hex>.bin`
  (`config` is force-symlinked to `config-default`).
- `<hex>` = the `BlobId`'s 32 raw bytes lowercase-hex (64 chars). Public `blob_id` is
  URL-safe-base64 of those bytes.
- Sidecars: `<hex>.meta` (standalone), `pools/<pool_id>/<hex>.meta` (pooled), YAML
  `BlobSidecar { blob_id, object_id, size, epochs, pool_id }`.
- No DB, no in-memory index, no locks — purely filesystem-derivable. A read needs only
  the dir + the `blob_id -> 32 bytes -> hex + ".bin"` rule.
- `sb-local` does NOT need to know the path scheme — it calls the engine. The path is
  documented only for understanding.

## Why this shape (decisions — do not relitigate)

- **GET + PUT both in `sb-local`** (not split, not daemon-hosted). A faucet-style
  standalone process. The daemon stays out of it.
- **Faithful Blob on PUT.** PUT goes through the engine's store path, which creates a
  REAL on-chain certified `Blob` (off-node held-key certify) + writes bytes to the
  shared fs dir. This is why it must be glibc (heavy walrus/Sui graph + RocksDB).
- **glibc, not musl** — `sb-local` lives in `localnet-tools` (pinned `ubuntu-24.04`
  glibc; RocksDB can't musl-link). Matches how MystenLabs/sui ships.
- **Drop the walrus CLI** as a client target (`walrus store/read` are node-direct and
  won't work nodeless). `sb-local` serves HTTP clients (curl/fetch/sites).

## Lifecycle (mirrors the localnet faucet)

`scripts/common/__sb-local-process.sh` — started on `localnet start`, stopped on
`localnet stop`, with its own bind address/port settings (`sb_local_host_ip` /
`sb_local_walrus_port`, default `localhost:45840` — the localnet slot of the Walrus
`458xx` range; see `docs/dev/PORT_ALLOCATION.md`). `localnet status` shows a
`Walrus API` line. Stop is `ps`-reap-safe (poll the re-check, like
`stop_walrus_relay_service_only`). The bind address and port are each their OWN
independent variable (the default may coincide with the faucet's pattern, but the
settings are separate). Gated on the `localnet` workdir +
`CFG_walrus_local_enabled=true`.

Startup ordering: `sb-local` needs the localnet RPC up + the walrus descriptor present,
so it starts **after** `deploy_walrus_localnet` in the exec path. `for_workdir` does
wallet/RPC setup once at server start and is shared via `Arc`, not per-request.

## Producer/consumer (localnet-tools — live)

- **Producer:** `chainmovers/sui-binaries`
  `.github/workflows/build-localnet-tools.yml` builds from suibase `pre-staging` with
  `cargo build --release --bin walrus-localnet-deploy --bin sb-local` on `ubuntu-24.04`
  (glibc) and packs both in the `localnet-tools` tarball. The precompiled
  `localnet-tools-v<n>` asset is cut when its version carrier
  (`triggers/localnet-tools/Cargo.toml`) changes — which the suibase `pre-staging.yml`
  lockstep drives automatically from `rust/localnet-tools/Cargo.toml`.
- **Consumer:** `scripts/defaults/consts.yaml` `localnet_tools_bin_names` carries both
  bins; install path `workdirs/common/bin/`. Auto-latest via
  `asset_name_filter: "localnet-tools"`. Source-build on dev via
  `scripts/dev/update-localnet-tools`; precompiled on main/staging.
- `staging.yml` **hard-fails** if `sb-local` is missing from the asset and validates the
  precompiled `sb-local` functionally (HTTP round-trip), gating staging → main.

## Wire compatibility — the acceptance criterion

GOAL: `sb-local` is a **drop-in replacement for the real Walrus aggregator + publisher
(`walrus daemon` mode)**. An existing agg/pub client (curl scripts, walrus-sites, any
HTTP client) must work against `sb-local` by **changing only the URL**.

**One port, both verbs = the real `daemon` topology.** Walrus's `daemon` subcommand
combines aggregator + publisher in ONE process, ONE bind address, ONE router carrying
both route sets (`ClientDaemon::new_combined`). So serving GET + PUT on one `sb-local`
port is faithful, not a shortcut — and it is drop-in for both deployment styles: a
client configured with SEPARATE `--aggregator`/`--publisher` URLs simply points both at
`sb-local`'s one URL.

**Relay is OUT of scope (and cannot be drop-in).** The upload-relay is a different
protocol (client encodes + relay fans slivers to storage NODES + client pays/signs)
that fundamentally needs real storage nodes — nodeless can't provide it. `sb-local` is a
drop-in for the AGGREGATOR + PUBLISHER only.

**The contract:**
- `PUT /v1/blobs` — body `application/octet-stream`; honor publisher query params
  `epochs` (default 1), `permanent`, `deletable` (deprecated/no-op — accepted),
  `send_object_to=<addr>` xor `share=true`. Respond `200` with the camelCase
  tagged-enum `BlobStoreResult` (`newlyCreated{ blobObject{ blobId, id, … }, … }` /
  `alreadyCertified{ blobId, endEpoch, event|object }`), and the `error` shape on
  failure. Params map to the engine store path (epochs → epochs; send_object_to/share →
  `PostStoreAction`).
- `GET /v1/blobs/{blob_id}` — `200` raw bytes + headers `ETag`(=blob_id),
  `Content-Type: application/octet-stream`, `X-Content-Type-Options: nosniff`; support
  HTTP `Range:` → `206`; `404` if absent. Plus `GET /v1/blobs/by-object-id/{id}`.
- **Quilt:** `PUT /v1/quilts` (`multipart/form-data`: each file field-name = the patch
  identifier; optional `_metadata` field = JSON `[{identifier, tags}]`; query `epochs` +
  `quilt_version` [V1 only]) → `200` `QuiltStoreResult { blobStoreResult: BlobStoreResult,
  storedQuiltBlobs: [{ identifier, quiltPatchId, range }] }`. Reads:
  `GET /v1/blobs/by-quilt-patch-id/{id}` + `GET /v1/blobs/by-quilt-id/{quilt_id}/{identifier}`
  → patch bytes + `X-Quilt-Patch-Identifier` header (+ tag headers);
  `GET /v1/quilts/{quilt_id}/patches` → `[QuiltPatchItem{ identifier, patch_id, tags }]`.
- `GET /status` — liveness.

**Deferred (truly-optional; don't break drop-in):** `/v1alpha` streaming/concat, JWT
auth. (multipart IS required for quilt PUT, so it is in scope.) The lifecycle ops
(`stat`/`extend`/`delete`/pools) aren't in the Walrus HTTP spec at all — they stay
Rust-API-only on the engine.

## Using the Rust mirror (current API)

```rust,no_run
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;
use walrus_core::encoding::Primary;

# async fn f() -> walrus_sdk::error::ClientResult<()> {
let client = WalrusLocalClient::for_workdir("localnet").await?;   // nodeless mock
let args = StoreArgs::default_with_epochs(5);

// Store + read — identical to walrus_sdk on a real network.
let results = client.reserve_and_store_blobs(vec![b"hello".to_vec()], &args).await?;
let blob_id = results[0].blob_id().unwrap();
let bytes = client.read_blob::<Primary>(&blob_id).await?;

// Byte-range read (slice a large blob without pulling it whole).
let slice = client
    .byte_range_read_client()
    .read_byte_range(&blob_id, 0, 4)
    .await?;

// Delete an owned blob.
let removed = client.delete_owned_blob(&blob_id).await?;
# Ok(()) }
```

Quilt is generic over the version (turbofish the version where the method is generic):

```rust,no_run
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;
use walrus_core::{
    encoding::{quilt_encoding::{QuiltStoreBlob, QuiltVersionV1}, EncodingType},
};

# async fn f() -> walrus_sdk::error::ClientResult<()> {
let client = WalrusLocalClient::for_workdir("localnet").await?;
let quilts = client.quilt_client();
let blobs = [QuiltStoreBlob::new(b"a", "alpha").unwrap()];

let quilt = quilts
    .construct_quilt::<QuiltVersionV1>(&blobs, EncodingType::RS2)
    .await?;
let result = quilts
    .reserve_and_store_quilt::<QuiltVersionV1>(quilt, &StoreArgs::default_with_epochs(5))
    .await?;
let _patches = client.quilt_client()
    .get_all_blobs(&result.blob_store_result.blob_id().unwrap())
    .await?;
# Ok(()) }
```

The same call sequences run verbatim against a real
`walrus_sdk::WalrusNodeClient` on testnet/mainnet — the caller dispatches by network.

## Crate layout

```
rust/walrus-local-sdk/
  src/lib.rs       WalrusLocalClient + LocalQuiltClient + LocalByteRangeReadClient
  src/localnet.rs  the LocalnetMockStore engine (+ pools, quilt, fs serving)
  src/compat.rs    WalrusApi (generic dispatch seam; real impl is pure forwarding)
  tests/           localnet_roundtrip, localnet_byte_range, localnet_pool,
                   localnet_pool_namespace, testnet_parity, common/ (shared parity body)

rust/localnet-tools/                       the bins crate (builds on walrus-local-sdk)
  src/bin/walrus_localnet_deploy.rs        the nodeless deploy tool
  src/bin/sb_local/{main,wire,quilt}.rs    the sb-local axum HTTP server
```

The Walrus/Sui deps in `localnet-tools` are git-pinned to the **same rev** as
`walrus-local-sdk` (rev `1049b56`, walrus **1.51.0** — matches what real networks run)
so the types unify across the path dependency.

## Tests + CI

- `cargo test --lib` — unit tests (pure byte-range validate/slice; no live localnet).
- `WALRUS_LOCALNET_TEST=1 cargo test --test <name>` — the live localnet suites
  (`localnet_roundtrip`, `localnet_byte_range`, `localnet_pool`,
  `localnet_pool_namespace`) against a `walrus_local_enabled` regen'd localnet.
- `WALRUS_TESTNET_TEST=1 WALRUS_TESTNET_CONFIG=<client_config.yaml> cargo test --test
  testnet_parity` — the **fund-gated** real-network parity test. It runs the SAME generic
  body (via `compat::WalrusApi`) against a real testnet `WalrusNodeClient`; self-skips
  when the env/config is absent.
- CI: `.github/workflows/walrus-localnet-integration.yml` (on dev) builds
  `walrus-local-sdk`, builds + stages `sb-local`, regens a localnet, runs the unit tests
  and the live localnet suites, and exercises the `sb-local` HTTP wire (curl PUT/GET +
  quilt) via `scripts/tests/050_walrus_tests/test_sb_local_http.sh`.

## Verified status (true now)

- **Byte-range read** implemented + tested: unit tests (pure validate/slice), an
  extensive live localnet integration test (`tests/localnet_byte_range.rs`: a 16 KB
  blob, many ranges + all error cases), AND the real-testnet parity test.
- **Quilt generic over `QuiltVersion`** (mirrors the SDK), live-verified on localnet.
- **Real-network drop-in parity verified LIVE** (`tests/testnet_parity.rs`): the same
  generic body ran against a real testnet `WalrusNodeClient` (fund-gated) —
  store/read/byte-range/dedup/delete of a real blob succeeded.
- **Real cross-environment blob ids:** a localnet id is bit-identical to what
  testnet/mainnet mint for the same bytes; quilt ids are real for free.
- The `sb-local` binary (`rust/localnet-tools/src/bin/sb_local/`, axum) is shipped in the
  precompiled `localnet-tools` asset (both bins) and validated by `staging.yml`.

## Naming

- Binary/process: **`sb-local`**. Asset: `localnet-tools` (umbrella — it can host more
  localnet HTTP services later, hence the general name).
</content>
</invoke>
