# `sb-local` — Suibase Localnet Walrus HTTP engine (implementation plan)

> Turnkey plan for a fresh autonomous session. Companion to
> `docs/dev/LOCALNET_WALRUS_PLAN.md` (the nodeless-localnet design) and
> `docs/dev/LOCALNET_WALRUS_FEATURE.md` (the WalrusStore feature). Read those first.

## Goal

`sb-local` ("suibase localnet") is a **standalone, long-running localnet-only HTTP
server** that exposes the **Walrus aggregator/publisher wire API** —
`GET /v1/blobs/{blob_id}` (read), `PUT /v1/blobs` (store), and the QUILT API
(`PUT /v1/quilts` + quilt reads — M5) — backed by the existing `walrus-store`
`LocalnetMockStore` engine, with blob_ids that EQUAL testnet/mainnet (M0). It gives polyglot clients
(`fetch`/`curl`/any HTTP, walrus-sites) a **switch-by-URL** store/read interface
against localnet, with **real on-chain Blob creation** on PUT.

It is managed by the suibase localnet lifecycle **exactly like the localnet faucet**
(start on `localnet start` when `walrus_enabled`, stop on `localnet stop`). It is a
**glibc** binary shipped in the `localnet-tools` asset (alongside
`walrus-localnet-deploy`). **The suibase-daemon is NOT involved** — no proxying,
no daemon dependency, so the daemon stays musl/lean.

## Why this shape (decisions already made — do not relitigate)

- **GET + PUT both in `sb-local`** (not split, not daemon-hosted). The user chose a
  faucet-style standalone process. The daemon stays out of it.
- **Faithful Blob on PUT.** PUT calls `LocalnetMockStore::store()`, which creates a
  REAL on-chain certified `Blob` (off-node held-key certify) + writes bytes to the
  shared fs dir. This is why it must be glibc (heavy walrus/Sui graph + RocksDB; see
  "Constraints").
- **Shared directory is automatic.** Because PUT/GET use `LocalnetMockStore`, blobs
  written via HTTP are byte-identical and readable via the Rust `WalrusStore` API and
  vice-versa — same dir, same `blob_id` derivation, no coordination. (Two front doors,
  one store.)
- **glibc, not musl** — `sb-local` lives in `localnet-tools` (already pinned
  `ubuntu-24.04` glibc; RocksDB can't musl-link). Matches how MystenLabs/sui ships.
- **Drop the walrus CLI** as a client target (`walrus store/read` are node-direct and
  won't work nodeless). `sb-local` serves HTTP clients (curl/fetch/sites).

## Prior-context facts (with file pointers — don't re-investigate)

**Engine (rust/walrus-store):**
- Construct: `WalrusStore::for_workdir("localnet").await?` (loads the workdir keystore +
  the descriptor `config/walrus-localnet.yaml` holding the held BLS committee key).
- `store(&self, bytes: &[u8], len) -> BlobHandle { blob_id, object_id }` (creates the
  on-chain Blob, writes bytes; dedups on content). `read(&self, blob_id) -> Vec<u8>`.
  Also `stat/extend/delete` + pools (NOT exposed over HTTP — see "Wire API scope").
- `src/localnet.rs` is the impl; `src/lib.rs` the public API; existing bin
  `src/bin/walrus_localnet_deploy.rs` (the deploy tool) is the model for a second bin.

**Shared blob directory (verified):**
- Bytes: `$HOME/suibase/workdirs/localnet/config/walrus-localnet-blobs/<hex>.bin`
  (`config` is force-symlinked to `config-default`). `localnet.rs:258, 703-705`.
- `<hex>` = the `BlobId`'s 32 raw bytes lowercase-hex (64 chars). `hex_key`,
  `localnet.rs:797-805`. Public `blob_id` is URL-safe-base64 of those bytes.
- Sidecars: `<hex>.meta` (standalone), `pools/<pool_id>/<hex>.meta` (pooled), YAML
  `BlobSidecar { blob_id, object_id, size, epochs, pool_id }` (`localnet.rs:129-147`).
- No DB, no in-memory index, no locks — purely filesystem-derivable. A read needs only
  the dir + the `blob_id -> 32 bytes -> hex + ".bin"` rule.
- `sb-local` just uses `LocalnetMockStore` — it does NOT need to know the path scheme;
  it calls `store()`/`read()`. (The path is documented only for understanding.)

**Walrus wire API to mimic (from MystenLabs/walrus `walrus-service` daemon):**
- Aggregator: `GET /v1/blobs/{blob_id}` → raw bytes body; metadata in headers
  (`ETag` = blob_id, `Content-Type`, `X-Content-Type-Options: nosniff`); supports HTTP
  `Range:`. `404` if absent. (`crates/walrus-service/src/client/daemon/routes.rs:84-108,315`.)
- Publisher: `PUT /v1/blobs` (body `application/octet-stream`) → `200` JSON
  `BlobStoreResult` (camelCase tagged enum: `newlyCreated { blobObject{ blobId,
  objectId, ... }, ... }` / `alreadyCertified { blobId, endEpoch, ... }` / `error{...}`).
  Query params (`PublisherQuery`): `epochs` (default 1), `permanent` (bool),
  `send_object_to=<addr>` xor `share=true`. (`routes.rs:1090,1566-1613`;
  `walrus-sdk/src/node_client/responses.rs:74`.) Local reference checkout:
  `/home/olet/repos/walrus-reference-main`.
- **Drop-in scope: match the real `daemon` (combined agg+pub) wire contract exactly** —
  see "Wire compatibility" below. Honor the publisher query params + the `BlobStoreResult`
  JSON shape + the aggregator headers. QUILT is IN SCOPE (M5); defer only truly-optional
  features (`/v1alpha` streaming/concat, JWT).

**Lifecycle to mirror — the localnet faucet:**
- `start_sui_faucet_process` / `stop_sui_faucet_process` + `update_SUI_FAUCET_PROCESS_PID_var`
  in `scripts/common/__globals.sh`; faucet runs at `http://localhost:9123`. Started/stopped
  from the localnet workdir exec path (`scripts/common/__workdir-exec.sh`) and shown in
  `localnet status`. Mirror this for `sb-local` (new `__sb-local.sh` or fold into
  `__walrus-localnet-deploy.sh`). Gate on `localnet` + `CFG_walrus_enabled=true`.

**Producer/consumer (localnet-tools, already live):**
- Producer: `chainmovers/sui-binaries` `.github/workflows/build-localnet-tools.yml` builds
  from suibase `pre-staging`, `cargo build --release --features localnet --bin <bin>` on
  `ubuntu-24.04` (glibc). Add `sb-local` as a second `--bin`.
- Consumer: `scripts/defaults/consts.yaml` `localnet_tools_bin_names` (currently
  `walrus-localnet-deploy`) → add `sb-local`. Install path `workdirs/common/bin/`.
  Auto-latest via `asset_name_filter: "localnet-tools"` (no force_tag). Source-build on
  dev via `scripts/dev/update-localnet-tools`; precompiled on main/staging.

## Constraints (hard)

- **glibc only.** `sb-local` links `walrus-store --features localnet` → walrus-sui +
  walrus-core + sui-types + RocksDB. RocksDB does NOT musl-link. So `sb-local` builds
  with the localnet-tools generic recipe (glibc, no musl) — never in the musl daemon.
- **localnet-only.** Gate everything on `localnet` workdir + `walrus_enabled=true`.
- **Bind address = its OWN, independent variable.** sb-local gets a distinct
  bind-address setting (e.g. `sb_local_bind` in suibase.yaml/consts.yaml). Mirror the
  faucet's configurable-bind PATTERN, but DO NOT reuse the faucet's variable or copy its
  value — the bind is set independently per process (the default may coincide, but it is a
  separate setting). Same for the port (its own variable).
- **Reuse, don't reimplement.** PUT/GET = `LocalnetMockStore::store()/read()`. Do NOT
  hand-roll blob_id derivation or register/certify — that's the brittle heavy logic the
  engine already owns.
- **Keys stay in the workdir.** `for_workdir("localnet")` uses the localnet keystore +
  the descriptor's held key. Localhost-bound, throwaway localnet keys — fine.

## Milestones (each: implement → red/green test → commit at green)

> Execution order is top-to-bottom: **M0 → M1 → M2 → M3 → M5 (quilt) → M4 (tests/docs, last)**.
> (The M4/M5 labels are historical — follow the doc order; tests/docs run last and cover quilt.)

- **M0 — real blob_id (cross-environment id equality with testnet/mainnet — DECIDED).** Make
  localnet blob_ids bit-identical to what testnet/mainnet mint for the same bytes (so clients can
  compute/verify ids and carry blob identity across networks — the closest UX/DX). In
  `LocalnetMockStore::store` + `store_pooled` (`localnet.rs:291-293, 536-538`), replace the
  `sha256(bytes)` Merkle-root STAND-IN with walrus-core's REAL encoder: the `EncodingConfig` the
  mock ALREADY builds for `encoded_size` (`EncodingConfig::new(n_shards).get_for_type(RS2)`) exposes
  `.compute_metadata(bytes) -> VerifiedBlobMetadataWithId` (`config.rs:102`; pure local compute, NO
  slivers retained, NO storage nodes). Use `verified.blob_id()` (`metadata.rs:373`) as the real
  blob_id, and build the on-chain `BlobObjectMetadata` from `verified` (confirm a
  `From<&VerifiedBlobMetadataWithId>` or root-hash accessor on `verified.metadata()`,
  `metadata.rs:378`). REORDER so `n_shards`/config is fetched BEFORE the id (the encoder needs it) →
  the dedup check then uses the real id. Cost: erasure-encodes each blob locally (CPU per store;
  fine for dev sizes). This also makes M5 quilt ids real for free (quilt_id = the packed blob's id).
  TEST: stored id == walrus-core's `compute_blob_id` for the same bytes (equality is by construction
  — same encoder; spot-check against a reference id, e.g. the `walrus` CLI's computed id for a file);
  round-trip read; on-chain Blob registers with the real id; the existing integration suite
  (`localnet_roundtrip`/`localnet_pool`) stays green. ~<1 day, foundational (do first).

- **M1 — the binary.** `rust/walrus-store/src/bin/sb_local.rs`, behind `localnet`
  feature (`required-features = ["localnet"]`). axum server (add `axum` to the localnet
  feature deps). Routes: `GET /v1/blobs/:id`, `PUT /v1/blobs`, `GET /status`. Holds one
  `WalrusStore::for_workdir("localnet")`. CLI args: `--port`, maybe `--bind`. Build with
  the documented env (`RUSTUP_TOOLCHAIN=1.96 LIBCLANG_PATH=... BINDGEN_EXTRA_CLANG_ARGS=...`).
  TEST (live, against a `walrus_enabled` regen'd localnet): `curl -X PUT --data-binary @file
  localhost:<port>/v1/blobs` → parse `blobId` → `curl localhost:<port>/v1/blobs/<blobId>`
  → bytes match. RED-check: a wrong blob_id → 404. Verify the on-chain Blob exists
  (object_id in the response) and the SAME blob reads back via the Rust `WalrusStore` API
  (interop / shared dir).
- **M2 — lifecycle.** Start/stop `sb-local` mirroring the faucet (`__globals.sh` +
  `__workdir-exec.sh` or a new `__sb-local.sh`); PID var; gate on localnet+walrus_enabled;
  port in `consts.yaml`/`suibase.yaml` (suggest default near RPC/faucet, e.g. `9124`,
  configurable). Add a `localnet status` line ("Walrus API : OK ( pid X )
  http://localhost:<port>"). TEST: `localnet start` (walrus on) starts it; `localnet stop`
  stops it; status reflects it; a regen restarts it. Reuse the wal-relay `ps`-reap-safe
  stop pattern (poll the re-check — see `stop_walrus_relay_service_only`).
- **M3 — producer + consumer.** Add `sb-local` to `build-localnet-tools.yml` (second
  `--bin`, in the tarball) and to `consts.yaml` `localnet_tools_bin_names`. Bump
  `rust/walrus-store/Cargo.toml` + the sui-binaries trigger to a new version (lockstep) so
  a fresh `localnet-tools-v<n>` is cut carrying both bins. Validate the precompiled path
  (the local-staging-branch lever from `project_localnet_tools_binaries`).
- **M5 — quilt parity (engine + HTTP).** Quilt IS in the real daemon HTTP API and matters for
  the product on top of suibase; it is feasible nodeless because quilt construction is 100%
  client-side pure compute (`walrus-core::encoding::quilt_encoding`, already linked at the same
  pinned rev — no new dep, no storage nodes). FIRST extend the engine (rust/walrus-store, mirror
  the M3 pool methods, ~1.5-2 days, comparable to pools):
    - `store_quilt(patches: [(identifier, bytes[, tags])], epochs)` → `QuiltConfigV1::get_encoder(
      EncodingConfigEnum, &blobs).construct_quilt()` (pure pack into ONE blob + embedded index)
      → run `quilt.into_data()` through the EXISTING `store()` body verbatim (after M0 this
      yields the REAL `blob_id` = `quilt_id`; reserve/register/held-key-certify/write fs) → return `quilt_id` + per-patch
      `QuiltPatchId = quilt_id ++ QuiltPatchInternalIdV1{start,end}.to_bytes()` (from the index).
    - `read_quilt_patch(quilt_id, patch_id)` / `read_quilt_blob(quilt_id, identifier)` →
      `read(quilt_id)` → `QuiltV1::new_from_quilt_blob(bytes, config)` →
      `get_blob_by_patch_internal_id` / `get_blobs_by_identifiers` (pure, no network).
    - Reuse walrus-core's quilt types verbatim (NO hand-rolled format): `QuiltStoreBlob`,
      `QuiltConfigV1`, `QuiltV1`, `QuiltPatchInternalIdV1`, `QuiltIndexV1`. Quilt is a normal
      Permanent `Blob`, so dedup/extend/delete/stat come free. Optional `<hex>.quilt` sidecar to
      cache the identifier→patch_id list for cheap listing.
  Then expose the quilt HTTP routes (above) in `sb-local` (multipart parse for PUT).
  TEST: `curl -F` multipart PUT /v1/quilts → `/v1/quilts/{id}/patches` → GET by-quilt-patch-id +
  by-identifier → bytes match; on-chain Blob exists; the quilt blob reads via the Rust API (interop).
- **M4 — tests + docs.** A `scripts/tests/050_walrus_tests/` test: start localnet+walrus,
  PUT+GET via curl, assert round-trip + on-chain Blob + Rust/HTTP interop. Update
  `LOCALNET_WALRUS_FEATURE.md` (HTTP facade section: what it is, the wire scope, the seam
  vs the Rust API, switch-by-URL note, and that the node-talking `@mysten/walrus` SDK
  targets testnet/mainnet, not nodeless localnet). Optionally a CI job (mirror
  walrus-localnet-integration.yml) exercising curl PUT/GET.

## Wire compatibility — the explicit acceptance criterion

GOAL: sb-local is a **drop-in replacement for the real Walrus aggregator + publisher
(`walrus daemon` mode)**. An existing Walrus agg/pub client (curl scripts, walrus-sites,
any HTTP client) must work against sb-local by **changing only the URL**.

**One port, both verbs = the real `daemon` topology (verified).** Walrus's `daemon`
subcommand combines aggregator + publisher in ONE process, ONE bind address, ONE router
carrying both route sets (`ClientDaemon::new_combined`, `crates/walrus-service/src/client/
daemon.rs:429,598-636`). So serving GET + PUT on one sb-local port is faithful, not a
shortcut — and it's drop-in for BOTH deployment styles: a client configured with SEPARATE
`--aggregator`/`--publisher` URLs simply points both at sb-local's one URL (GET routes serve
the aggregator calls, PUT the publisher calls; no path/method conflict). The 2-process/2-port
split in public Walrus is an auth/funds/scaling choice, irrelevant on localhost. (If literal
2-URL separation is ever wanted, ONE sb-local process can bind two listeners — unnecessary
for drop-in.)

**Relay is OUT of scope (and cannot be drop-in).** The upload-relay is a different protocol
(client encodes + relay fans slivers to storage NODES + client pays/signs) that fundamentally
needs real storage nodes — nodeless can't provide it. sb-local is a drop-in for the
AGGREGATOR + PUBLISHER only; on localnet, writes go through the publisher path.

**Match the contract EXACTLY (the bar is "wire-compatible," not "bare PUT/GET"):**
- `PUT /v1/blobs` — body `application/octet-stream`; honor publisher query params `epochs`
  (default 1), `permanent`, `deletable` (deprecated/no-op — accept it), `send_object_to=<addr>`
  xor `share=true`. Respond `200` with the camelCase tagged-enum `BlobStoreResult`
  (`newlyCreated{ blobObject{ blobId, id, ... }, ... }` / `alreadyCertified{ blobId, endEpoch,
  event|object }`), and the `error` shape on failure. Cross-check FIELD-FOR-FIELD against
  `walrus-sdk/src/node_client/responses.rs:74` + `routes.rs:1566-1613` in the reference checkout.
  Map params → `LocalnetMockStore::store` (epochs → epochs; send_object_to/share → PostStoreAction).
- `GET /v1/blobs/{blob_id}` — `200` raw bytes + headers `ETag`(=blob_id),
  `Content-Type: application/octet-stream`, `X-Content-Type-Options: nosniff`; support HTTP
  `Range:` → `206`; `404` if absent (`routes.rs:239,315`). Optionally `GET /v1/blobs/by-object-id/{id}`.
- **Quilt (IN SCOPE for parity — engine work in M5):** `PUT /v1/quilts`
  (`multipart/form-data`: each file field-name = the patch identifier; optional `_metadata`
  field = JSON `[{identifier, tags}]`; query `epochs` + `quilt_version` [V1 only]) → `200`
  `QuiltStoreResult { blobStoreResult: BlobStoreResult, storedQuiltBlobs: [{ identifier,
  quiltPatchId, range }] }`. Reads: `GET /v1/blobs/by-quilt-patch-id/{id}` +
  `GET /v1/blobs/by-quilt-id/{quilt_id}/{identifier}` → patch bytes + `X-Quilt-Patch-Identifier`
  header (+ tag headers); `GET /v1/quilts/{quilt_id}/patches` → `[QuiltPatchItem{ identifier,
  patch_id, tags }]`. Cross-check `responses.rs:215-223`, `client_types.rs:781-799`,
  `routes.rs:96-104,1732`.
- `GET /status` — liveness.

**Defer only truly-optional (TODO; don't break drop-in):** `/v1alpha` streaming/concat, JWT
auth. (multipart IS required for quilt PUT, so it's in scope.) The lifecycle ops
(`stat`/`extend`/`delete`/pools) aren't in the Walrus HTTP spec at all — they stay Rust-API-only.

**Drop-in test (M4):** point an existing agg/pub client (or replay the exact wire contract via
curl) at sb-local; assert the request/response matches the real daemon, AND that a blob PUT over
HTTP is readable both via GET and via the Rust `WalrusStore` API (interop).

## Risks / watch-outs

- **Wire fidelity.** Keep `BlobStoreResult` camelCase + the tagged-enum shape so existing
  Walrus HTTP clients parse it. Cross-check against `responses.rs:74` in the reference checkout.
- **blob_id fidelity (PRE-EXISTING; affects ALL blobs incl. quilts; DECIDED → done in M0).** The mock
  derives `blob_id` from `sha256(bytes)` as a Merkle-root STAND-IN (`localnet.rs:289-293`), not the
  real Blake2b root over erasure-encoded symbols. So mock blob_ids/quilt_ids are NOT bit-identical to
  real Walrus for the same content. Round-trip WITHIN the mock is fully faithful (store → id →
  read/list/by-patch all consistent); only cross-environment id-EQUALITY differs (same files →
  different id on real Walrus vs the mock). To get TRUE id parity, switch the mock to walrus-core's
  real `BlobEncoder::compute_metadata` (`blob_encoding.rs:406` — returns the id WITHOUT keeping
  slivers, so still nodeless/pure-compute) for both plain blobs AND quilts, at the cost of
  erasure-encoding every blob locally (CPU/time per store). Orthogonal to quilt; underpins both.
  **DECIDED (owner, 2026-06-19): YES — localnet ids must EQUAL testnet/mainnet ids (closest UX/DX:
  a client can compute/verify ids and carry blob identity across networks). M0 (below) implements
  it; M5 quilt ids then become real for free. Confirmed feasible + small: the `EncodingConfig` the
  mock already builds exposes `.compute_metadata(bytes)` (config.rs:102) — pure local compute.**
- **Port allocation/conflict.** Pick a stable default; make it configurable; show in status.
- **Startup ordering.** `sb-local` needs the localnet RPC up + the walrus descriptor present
  (i.e. after the deploy hook). Start it AFTER `deploy_walrus_localnet` in the exec path.
- **`for_workdir` cost.** Constructing `WalrusStore` does wallet/RPC setup; do it ONCE at
  server start, share via `Arc`, not per-request.
- **Prereq:** start this AFTER `08c0a913` (wal-relay + toolchain) lands on main, so it
  builds on a clean main + the latest localnet-tools pipeline.

## Naming
- Binary/process: **`sb-local`**. Asset: `localnet-tools` (umbrella). It can host more
  localnet HTTP services later (hence the general name).
