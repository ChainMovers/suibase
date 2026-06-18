# Nodeless Local Walrus on Suibase Localnet + Workdir-Aware `WalrusStore` — Implementation Plan

**Status:** Draft working plan · **Gate-0 verdict:** ✅ **PASS — GO** (empirically confirmed on
suibase localnet, 2026-06-17).

> This is the **working implementation plan** for a new Suibase feature, authored on the
> `feature/localnet-walrus` branch. It is the precursor to the generic, end-user-facing
> `docs/dev/LOCALNET_WALRUS_FEATURE.md` that lands as the **M5** deliverable. It mirrors the
> structure of `docs/dev/WALRUS_RELAY_FEATURE.md`.
>
> Product context, rationale, and the pinned decisions (WS1–WS7) live in the private planning
> brief `suibase-localnet-walrus-spec.md` (Suiftly). That brief is intentionally **not** copied
> into this repo; this plan is generic Sui+Walrus infrastructure with no downstream-product content.

## Overview

Two coupled capabilities:

1. **Nodeless local Walrus** on a Suibase localnet: real Walrus `Blob`/`Storage`/`StoragePool`
   objects created via Sui PTBs against genuinely-published Walrus Move packages, with blob
   **bytes stored on the local filesystem** (keyed by `blob_id`). **No storage nodes, no RocksDB,
   no erasure coding.** The single operation that normally requires a live node —
   `certify_blob` — is satisfied **off-node** by holding the committee's BLS secret key and
   self-signing the certificate.

2. **A workdir-aware `WalrusStore` Rust client** that selects a localnet **mock** implementation
   on `localnet` and the **real `walrus-sdk`** on `testnet`/`mainnet`, via
   `WalrusStore::for_active_workdir()` — so app code is network-agnostic.

The mock's home is a **sibling crate** (e.g. `rust/walrus-store`, Apache-2.0) that depends on the
suibase helper crate (`rust/helper`, crate name `suibase`) **only behind a `localnet`/`mock`
feature**. The default build pulls only `walrus-sdk`. Nothing enclave-side links the mock or
suibase; a `cargo tree` CI check enforces this (WS7).

### Why nodeless certify is possible (the crux — independently source-verified)

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
  and self-certify.

The signing path reuses `walrus-core` verbatim (no hand-rolled BCS): build a `Confirmation` and sign
with `ProtocolKeyPair::sign_message` (`keys.rs:261-273`); the scheme is fastcrypto BLS12381 **min_pk**,
matching the on-chain native `bls12381_min_pk_verify`.

## Gate-0 Results (empirical — PASS) {#gate-0-results}

Executed 2026-06-17 against a live suibase localnet (Sui `1.73.1-ff1fe0ec`, the exact version the
Walrus reference pins) using binaries built from `walrus-reference-main @ 1049b56`. Build dependency
resolved without sudo: `libclang.so` from the `libclang` pip wheel + GCC freestanding headers via
`BINDGEN_EXTRA_CLANG_ARGS`. Two independent proofs:

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

**Two empirical findings that refine the plan (both mechanical, neither a blocker):**

- **Committee staking is a real step `walrus-deploy` does not perform.** `deploy-system-contract` and
  `generate-dry-run-configs` leave the committee **empty** (`members: []`); the node is only registered
  + staked by `register_committee_and_stake` + `end_epoch_zero` (`walrus-sui` test-utils). Layer A (M1)
  must perform this step itself (a small helper bin, or replicate those two functions) — the
  node-oriented `generate-dry-run-configs` is not sufficient.
- **Localnet has read-after-write lag.** After `initiate_epoch_change`, the fullnode (`:9000`) takes a
  few seconds to reflect the new epoch/committee — the committee read returned empty twice, then
  `epoch=1 members=1` on the third read (~4s). Poll `current_committee()` until it advances rather than
  reading once.

## Gate-0 Spike (throwaway — must PASS before building the feature)

**Hard GO/NO-GO:** if an off-node held-key signature will not pass `certify_blob`, **STOP**.
Fallback: register-only locally + use testnet for renewal/extend.

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
pubkeys compressed G1 (48 B), signatures compressed G2 (96 B), IETF DST
`BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_` (baked into fastcrypto/blst — not verbatim in the repo;
the sign→verify round trip is the proof). **Do not hand-roll BCS** — use `Confirmation::new` +
`ProtocolKeyPair::sign_message`.

**Recipe (PASS = all three checks below):**

1. **Build:** `cargo build --release -p walrus-service --bin walrus-deploy --features deploy`
   (the `deploy` feature pulls `walrus-sui/test-utils`, required for deploy + dry-run-configs) and
   `--bin walrus`.
   **Build dependency:** the whole Sui/Walrus crate graph needs RocksDB →
   `zstd-sys`/`bindgen` → **libclang** (`sudo apt-get install -y clang libclang-dev`).
2. **Localnet:** `~/suibase/scripts/localnet start`. Faucet `http://127.0.0.1:9123/gas`; fullnode RPC
   **`http://localhost:9000`** (confirmed `defaults/localnet/suibase.yaml:158`).
3. **Deploy contract (N=1, deterministic):**
   `walrus-deploy deploy-system-contract --working-dir ./wd --sui-network localnet --contract-dir ./contracts --n-shards 1 --host-addresses 127.0.0.1 --deterministic-keys --with-wal-exchange`.
   Capture `package_id / system_object / staking_object / exchange_object` and `nodes[0].keypair`
   from `./wd/testbed_config.yaml`.
4. **Stake + end epoch 0 (the step `deploy-system-contract` omits):**
   `walrus-deploy generate-dry-run-configs --working-dir ./wd` → runs `register_committee_and_stake`
   + `end_epoch_zero` (`system_setup.rs:535-546`). Confirm the epoch advanced past 0 and the single
   node is the live committee. *(This correction is critical: deploy alone leaves an empty committee
   and certify would abort.)*
5. **Fund (real WAL):** `walrus generate-sui-wallet --sui-network localnet --use-faucet`, then
   `walrus get-wal --exchange-id <exchange_object> --amount <mist>` (1:1 default rate; exchange must
   be WAL-funded by deploy).
6. **Spike bin** (throwaway; depends on `walrus-core` + `walrus-sui`/`sui-sdk` + `fastcrypto`):
   - **reserve** → `system::reserve_space(System(mut), encoded_size:u64, epochs_ahead:u32, &mut Coin<WAL>) -> Storage`.
     Compute `encoded_size` via `encoding::encoded_blob_length(size, enc, n_shards)` or over-reserve.
   - **register** → pick `root_hash`, `encoding_type`, `size`; `blob_id = derive_blob_id(...)`;
     `system::register_blob(System(mut), Storage, blob_id:u256, root_hash:u256, size:u64, encoding_type:u8, deletable=false, &mut Coin<WAL>) -> Blob`.
   - **sign off-node** → `kp = ProtocolKeyPair::from_str(<testbed nodes[0].keypair>)`;
     `signed = kp.sign_message(&Confirmation::new(epoch, BlobId(blob_id), Permanent))`;
     assert `signed.serialized_message` equals the 40-byte layout above;
     `agg = BLS12381AggregateSignature::aggregate(&[signed.signature])`; `bitmap = vec![0x01]`.
   - **certify** → `system::certify_blob(System(immut), Blob, agg_bytes, bitmap, msg)`.
     **PASS #1** = `BlobCertified` emitted, `certified_epoch` set.
   - **extend** → `system::extend_blob(System(mut), Blob, epochs:u32, &mut Coin<WAL>)`.
     **PASS #2** = succeeds (extend hard-requires `assert_certified_not_expired`).
   - **fs roundtrip** → write bytes to `<store>/<blob_id_hex>`, read back, assert equality.
7. **Regen survival:** `~/suibase/scripts/localnet regen`, re-run steps 3–6 (regen wipes the chain +
   published-data). **PASS #3** = the full flow reproduces deterministically.

Any `ESigVerification` on certify = **HARD NO-GO**.

## Architecture

```
            WalrusStore::for_active_workdir()
                       |
            +----------+-----------+
            |                      |
      localnet/mock           default (real)
            |                      |
   LocalnetMockStore         RealWalrusStore (walrus-sdk)
            |
      +-----+------+
      | suibase     |  discovery (sync): package_id, published_new_objects,
      | Helper      |  client_address, rpc_url, keystore_pathname
      +-----+------+
            |
   PTBs: reserve_space -> register_blob -> certify_blob -> extend_blob
            |                                 (held BLS key, off-node sign)
   bytes on filesystem (keyed by blob_id)
```

- **Layer A (bash):** publishes Walrus into the freshly-regenerated localnet and records the minted
  object IDs + held committee key.
- **Layer B (Rust `WalrusStore`):** workdir-aware client; the mock builds PTBs and signs certs
  off-node; the real impl delegates to `walrus-sdk`.

### Integration map — constrained vs free values (store path)

- **FREE:** `blob_id` (must match the register arg and the signed message), `root_hash`, `size`,
  `epochs_ahead`.
- **Constrained:** `encoding_type` (valid variant); `encoded_size ≥ encoding::encoded_blob_length(size,enc,n_shards)`;
  WAL coin balance `≥ reserve+write+extend prices`; `cert_epoch == live system epoch`; BLS cert
  (held key, min_pk, 40-byte message, `bitmap=[0x01]`); use `deletable=false` (Permanent) so the
  signed message needs no blob object id.

## Suibase integration points (verified line refs)

| # | File | Change |
|---|---|---|
| 1 | `scripts/common/__globals.sh:2578-2588` `is_walrus_supported_by_workdir()` | currently hard-codes `testnet`/`mainnet`; add a `localnet` arm at line 2583 (gate behind a localnet enable flag). |
| 2 | `scripts/common/__globals.sh` `repair_walrus_config_as_needed()` / `repair_walrus_rpc_urls_as_needed()` | add a localnet arm; **skip** the static per-field repair (IDs come from deploy, not the template); localnet rpc → `http://localhost:9000`. Stay defensive (`return 0` on missing files). |
| 3 | `scripts/common/__walrus-localnet-deploy.sh` (**new**) | `deploy_walrus_localnet()`: run `walrus-deploy deploy-system-contract` + `generate-dry-run-configs`, capture object IDs + `nodes[0].keypair`, write them into `workdirs/localnet/config-default/walrus-config.yaml`. Idempotent; **no node/process management.** |
| 4 | `scripts/common/__workdir-exec.sh` regen flow | insert `deploy_walrus_localnet; repair_walrus_config_as_needed localnet` after the Sui wipe and before `start_all_services`, **non-fatal** (`warn_user` on failure). |
| 5 | `scripts/defaults/localnet/suibase.yaml:151-154` | walrus ports are `~` today; set non-colliding localnet values (e.g. proxy 45851 / local 45801 / metrics 45811) if the relay path is reused, else leave nodeless. |
| 6 | `scripts/templates/localnet/config-default/walrus-config.yaml` (**new**) | mirror the testnet/mainnet shape; context `localnet`; `rpc_urls: [http://localhost:9000]`; system/staking/exchange/package ids + committee-key handle as placeholders filled at deploy. |
| 7 | `rust/walrus-store` (**new sibling crate**) | `WalrusStore` trait + `LocalnetMockStore` (behind `localnet`/`mock`) + `RealWalrusStore` (default, `walrus-sdk`) + `for_active_workdir()`. |
| 8 | `scripts/tests/050_walrus_tests/test_localnet_walrus_*.sh` (**new**) | mirror existing tests: deploy-on-regen, store/read/extend/delete round-trip, pool lifecycle, regen survival. |
| 9 | `docs/dev/LOCALNET_WALRUS_FEATURE.md` (**new, M5**) | generic end-user/dev doc (deploy recipe, held-key model, selection, funding, feature flags). |

Conventions (`CONTRIBUTING.md`): work on `dev`-derived branch; `shellcheck`; `export -f`; UPPERCASE
globals; must pass `scripts-tests` + `rust-tests`.

## Milestones (each ends in a green gate)

- **M0 — Gate-0 spike (throwaway):** prove off-node certify per the recipe above; delete after GO.
- **M1 — Nodeless deploy (Layer A, bash):** `deploy_walrus_localnet()` produces a valid
  `walrus-config.yaml` (ids + committee key); idempotent; re-runs clean. Edits #1–#6 above.
- **M2 — `WalrusStore` mock store/read/stat/extend/delete:** sibling crate; `LocalnetMockStore`
  builds reserve/register/certify PTBs (arg order mirrors
  `crates/walrus-sui/src/client/transaction_builder/owned_blob_ops.rs`), holds the committee key,
  writes bytes to a store dir; discovery via `suibase::Helper`. Objects are **real + certified**
  (verified via Sui reads).
- **M3 — Pool ops (DD-D8):** add `create_pool`/`register_pooled`/`delete_pooled`
  (`system.move:216/238/264`); `create_storage_pool` returns `StoragePool` by value →
  `public_transfer` to sender; delete needs no certify.
- **M4 — real-sdk impl + `for_active_workdir()`:** `RealWalrusStore` behind the default feature
  wrapping `walrus-sdk`; selection reads the active suibase workdir; a testnet smoke store/read passes.
- **M5 — Suibase wiring + regen + tests + docs:** idempotent re-deploy on each regen (chain wiped in
  `__workdir-exec.sh`); new `scripts/tests/050_walrus_tests/` cases; Rust integration test gated on
  `localnet`; `cargo tree` CI assertion that the enclave graph has no `suibase`; write
  `docs/dev/LOCALNET_WALRUS_FEATURE.md`.
- **M6 — Downstream consumer validation (out of scope here):** confirm the default build excludes
  mock+suibase and the consumer uses `for_active_workdir()` only behind its own mock feature.

## Risks

1. ~~fastcrypto min_pk DST not verbatim in-repo~~ — **RESOLVED** (Gate-0): the sign→verify round trip
   passed (Walrus's own `test_register_certify_blob` + the localnet spike). Always sign via
   `ProtocolKeyPair::sign_message` so the DST is whatever fastcrypto uses.
2. **`deploy-system-contract` / `generate-dry-run-configs` do NOT stake the committee** (confirmed
   empirically — committee stays `members: []`) → Layer A must call `register_committee_and_stake` +
   `end_epoch_zero` itself, then **poll `current_committee()`** until the new epoch is reflected
   (localnet read-after-write lag, ~seconds).
3. **min_pk / key-format footgun** → load via `ProtocolKeyPair::from_str` (handles the `0x04` flag);
   never raw `from_bytes`.
4. **`cert_epoch` must equal the live epoch at submission** → read epoch just before signing and
   submit promptly; lengthen epoch duration for a stable spike.
5. **WS7 enclave exclusion:** `suibase` pulls `sui-types` (path into `workdirs/active/sui-repo`) →
   sibling crate + `localnet`/`mock` feature gate + `cargo tree` CI assertion.
6. **`encoded_size`** must cover the contract-computed encoded length or register aborts
   `EResourceSize`.
7. **Deletable blobs** need the blob object id in the signed persistence byte (unknown until register
   executes) → use Permanent in the store path; split into register-PTB then sign+certify-PTB for
   deletable.

## Open questions

- Exact fastcrypto DST at the pinned rev — resolve empirically in M0.
- Whether `update_walrus` fetches a **localnet** walrus binary (today it's testnet/mainnet only) —
  confirm before wiring M1.
- Auto-deploy Walrus on **every** regen vs only when a localnet-walrus enable flag is set
  (time/cost tradeoff).
- Whether `WalrusStore` should reuse the same `sui-types` path `suibase` pins (avoids version skew,
  inherits the workdir-build requirement) or depend on `sui-sdk`/`sui-types` directly.
- Whether the enclave needs any discovery API at runtime or only at provisioning (if
  provisioning-only, the sibling crate can be test/build-only and fully excluded from the artifact).
- Exchange WAL seed amount on localnet (bounds max convertible per session).

## Reference commit pins

- Walrus reference checkout: `/home/olet/repos/walrus-reference-main` @ `1049b56` (record the exact
  rev so localnet contracts match the `walrus-sdk` used by the real impl — Q4).
- Suibase: `feature/localnet-walrus` off `dev`.
