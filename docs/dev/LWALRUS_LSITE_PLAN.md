# lwalrus / lsite Plan — localnet `walrus` & `site-builder` CLI parity

Status: **Phase 1 (`lwalrus`) MVP IMPLEMENTED** (2026-06-22): a shim binary in
`rust/localnet-tools` (`store`/`read`/`blob-status`/`delete` over WalrusLocalClient),
the `scripts/lwalrus` wrapper + `__lwalrus-exec.sh` gate, added to
`localnet_tools_bin_names`. **Phase 2 (`lsite` + portal) DEFERRED** ("revisit if
anyone cares"). Feasibility from workflows wf_5875ba7a + wf_b784f37e.

PARITY MODEL (decided 2026-06-22): NOT byte-exact `--help` (lwalrus is a subset, and
the localnet stack may be pinned to a different walrus rev than the shipped binary).
Instead lwalrus carries an explicit "Not supported for localnet:" help section (the
single source of truth), invoking anything unsupported prints "Not supported for
localnet", and a fund-free always-on test (`scripts/tests/050_walrus_tests/test_lwalrus_parity.sh`)
SEMANTICALLY compares only the SUPPORTED surface vs the shipped `walrus`, ignoring the
unsupported list — flagging drift (new/removed/changed) among supported commands.

DONE — `scripts/dev/update-walrus-pin` (detect-by-default / `--apply`) AND the v1.50.0 re-pin.
Target rev = the commit of the `walrus` binary `testnet update` installs (`walrus --version`
→ resolved via the `~/repos/walrus-reference-main` checkout). Detect: exit 0 in-sync / 3
re-pin-available / 1 error. `--apply` rewrites the walrus `rev =` lines, **auto-aligns the
`sui-types` tag to walrus's own sui dep**, re-vendors `embedded-contracts/` + regenerates
CONTRACTS.sha256, rebuilds, runs verify + the lwalrus parity test, fail-loud.

Re-pin to v1.50.0 (`1049b56` → `dac31b8`) needed TWO non-obvious fixes the experiment
surfaced (so a re-pin is NOT just a rev bump):
1. **sui-types `testnet-v1.73.1` → `testnet-v1.73.0`** — walrus v1.50.0 pins sui at 1.73.0;
   the mismatch put two `sui_types` in the graph (18 `ObjectID`/`EventID` errors). The
   script now auto-aligns this.
2. **`walrus_localnet_deploy` materializes embedded contracts BESIDE `deploy_dir`, not
   inside it** — v1.50.0's `create_and_init_system` copies `contract_dir` INTO
   `deploy_directory` (recreating it), wiping a nested contract dir.
Validated end-to-end on localnet (build, verify, v1.50.0 deploy, full parity incl. round-trip).

## Goal

Add workdir-aware `lwalrus` and `lsite` to the **localnet**, mirroring
the existing `twalrus/tsite` (testnet) and `mwalrus/msite` (mainnet) wrappers.
Those wrap the **real** `walrus`/`site-builder` binaries config-pointed at the
real network. Localnet was excluded historically (`is_walrus_supported_by_workdir`
is testnet/mainnet only; commit `84d77137` skips the walrus binary fetch for
devnet/localnet) because the localnet has no storage nodes.

## Why this is feasible now

The expensive parts already exist and are validated:
- Walrus Move contracts published on localnet (`walrus-localnet-deploy`, held-key N=1 committee).
- `sb-local`: HTTP Walrus aggregator+publisher on localnet (`/v1/blobs`, `/v1/quilts`, `/status`).
- `WalrusLocalClient` (`rust/walrus-local-sdk`) + `@suibase/walrus-local`: drop-in mirror of `walrus_sdk`'s `WalrusNodeClient`, SDK-parity validated.
- The `localnet-tools` precompiled pipeline (builds/publishes/fetches `sb-local` + `walrus-localnet-deploy`).

So the remaining work is a **CLI frontend over things that already work**, not new storage infrastructure.

## Architecture decision

**Build a new binary `lwalrus` in `rust/localnet-tools`** (rides the existing
precompiled-asset pipeline next to `sb_local` + `walrus_localnet_deploy`). It is
a thin shim: parse the `walrus` CLI arg surface, dispatch storage ops to
`WalrusLocalClient`/`sb-local`, emit `walrus`-compatible output.

- **`lwalrus` MUST be a shim, not a config-pointed real binary.** Confirmed: the
  real walrus `ClientConfig` has only `contract_config / rpc_urls /
  communication_config` — **no aggregator/publisher backend mode**. The real CLI
  always talks directly to storage nodes, which the localnet lacks. The t/m
  "config-only" trick is provably non-viable here.
- **`lsite` needs ~zero new code.** `site-builder` embeds no walrus client — it
  shells out (`walrus json <input>`, `site-builder/src/walrus.rs:275-277`). So
  `lsite` = the **real** `site-builder` binary + a localnet `sites-config.yaml`
  whose `walrus_binary` points at `lwalrus`. It rides entirely on `lwalrus`'s
  `walrus json` compatibility.
- **The portal** (to *view* sites) = run the **real portal server** (Bun/Node,
  `/tmp/walrus-sites/portal/server`) config-pointed at localnet. Not the worker
  (build-time config), not sb-local (it's a blob aggregator, not an on-chain
  site resolver).

## `lwalrus` command map (3 buckets)

**Bucket A — storage ops → the shim's real job** (map to `WalrusLocalClient`):
`store`, `read`, `blob-status`, `store-quilt`, `read-quilt`, `delete`.
(`read-quilt` by *tag* must go through the Rust client, not sb-local HTTP, which is identifier-only.)

**Bucket B — on-chain / local-compute** (route through the shim too — the real
binary aborts on missing node config before reaching them): `info`, `blob-id`,
`convert-blob-id`, `list-blobs`, `list-patches-in-quilt`, `extend`,
`burn-blobs`, `fund-shared-blob`, `share`, `*-blob-attribute`, `generate-sui-wallet`.

**Bucket C — honest exceptions** (print a friendly "not applicable on
localnet", do not fake): `stake` / `request-withdraw-stake` / `withdraw-stake` /
`list-staked-wal` (no node pools), `node-admin`, `health` (no nodes),
`pull-archive-blobs` / `blob-backfill` (operator tools), and **daemon mode**
(`aggregator` / `publisher` / `daemon` — sb-local already *is* this; relay is `wal-relay`).
`get-wal` is deferred-not-impossible (localnet exchange exists; low priority).

Note: on a 1-epoch localnet the clock doesn't advance, so blobs effectively never
expire — documented behavior, not a bug; the test harness controls epochs.

## THE risk: JSON output parity (the one discipline that de-risks everything)

`site-builder` parses `walrus`'s `BlobStoreResult` / `QuiltStoreResult` JSON
**exactly**, and there is a trap: site-builder uses a **hand-maintained, older
*subset*** of that schema (`site-builder/src/walrus/output.rs` — no `Error`
variant, no `shared_blob_object`), and it uses **`store --dry-run`** (a *distinct*
output type) during resource-diffing. So:

- **Parity must be EMPIRICAL, not assumed.** Do NOT rely on "reuse the SDK serde
  types." First task of any `lsite` work: capture golden JSON from the **real**
  `walrus` binary (exact fetched version) for `store` / `store-quilt` /
  `store --dry-run` / `read`, and byte-diff against what `lwalrus` emits AND what
  the installed `site-builder` deserializes.
- **Version-lock the trio** (walrus rev `1049b56`, `walrus-local-sdk`,
  `site-builder`). The `site-builder` fetch is currently **unpinned** in
  `consts.yaml` — durable `lsite` needs pinning + a CI golden-parity test.
- Implement `--dry-run store-quilt` (returns encoded sizes/costs without storing).

`lwalrus` standalone (Bucket A/B) carries **none** of this tax — it's purely additive.

## `lsite` end-to-end pieces (beyond `lwalrus`)

1. **Sites Move package on localnet — MEDIUM (the real gate).** Vendor
   `/tmp/walrus-sites/move/walrus_site` into `embedded-contracts/`, extend
   `walrus_localnet_deploy` to publish it and record `sites_package_id` in the
   descriptor. **Gotcha:** `walrus_site/Move.toml` depends on `suins` via MVR,
   which doesn't exist on localnet. `site.move` only uses suins for the optional
   reverse-lookup, but it must still *link*. Resolve by vendoring a minimal SuiNS
   core package on localnet, or patching the Move.toml to a vendored/stub path.
   If this turns into a Move yak-shave, **stop and reassess** — `lsite` is not
   worth a multi-day Move fight.
2. **site-builder binary + localnet `sites-config.yaml` — SMALL.** Fetch the real
   binary for localnet (relax the binary-fetch guard for localnet only — do NOT
   relax `is_walrus_supported_by_workdir` for the real-binary exec path); generate
   a localnet sites-config at regen with explicit `package=<sites_package_id>`
   (no MVR default resolution on localnet), `rpc=http://localhost:9000`, localnet
   wallet, `walrus_binary=<lwalrus>`. After this, `lsite publish` works **headless**
   (useful for tests with no browser).
3. **Portal — MODERATE.** Real portal server, config-pointed: `RPC_URL_LIST=http://localhost:9000`,
   `AGGREGATOR_URL_LIST=http://localhost:45840` (sb-local), `ORIGINAL_PACKAGE_ID=<sites pkg>`,
   `B36_DOMAIN_RESOLUTION_SUPPORT=true`, object-id/base36 access only
   (`http://<b36oid>.localhost:3000/`). Friction: the portal unconditionally
   constructs `SuinsClient` at startup (`rpc_selector.ts`) — pass a dummy
   `SUINS_CLIENT_NETWORK=testnet` (never resolved on the b36 path) or apply a small
   lazy-init patch; tune `PORTAL_DOMAIN_NAME_LENGTH` for localhost; add **CORS** on
   sb-local (none today) or reverse-proxy same-origin; raise `AGGREGATOR_REQUEST_TIMEOUT_MS`
   for cold-start blobs. Adds a Bun runtime dependency + a portal start/stop lifecycle.
4. **SuiNS / human-readable names — OUT OF SCOPE.** Object-id (base36) access only.

Rough sizing for `lsite` on top of `lwalrus`: ~3–5 focused days (pkg publish incl.
suins dep ~1–2d; site-builder+config ~0.5–1d; portal config+lifecycle+CORS/timeout ~1d; e2e+docs ~1d).

## `lsite` value verdict (why defer, not skip)

Real parity payoff but **narrow**. It's the *real* site-builder + a *real*
portal — true behavioral parity (same PTBs, resource/route/redirect handling),
not a mock. That's valuable for developers building Walrus **Sites** tooling who
want a fully offline, faucet-free, rate-limit-free `publish → view → inspect →
regen` loop. But the **majority** of suibase users just want a local chain +
blob store, which `lwalrus` already delivers. `lsite` roughly doubles the moving
parts (sites Move pkg + suins dep + a Node/Bun portal with CORS/subdomain/timeout
quirks) and the things that can silently break on a regen.

## Phased plan

**Phase 1 — `lwalrus` (ship now; clean, low-maintenance):**
- `lwalrus` Rust shim in `rust/localnet-tools/src/bin/` — arg surface + dispatch +
  the `walrus json` decoder; Bucket A over `WalrusLocalClient`, Bucket B handlers,
  Bucket C friendly stubs. (~300–600 LOC + handlers.)
- `__lwalrus-exec.sh` + `scripts/lwalrus` wrapper (mirror `twalrus`, `WORKDIR=localnet`).
- A **localnet-only gate** for `lwalrus` (do NOT relax the t/m guard for the real-binary path).
- `WalrusLocalClient::for_workdir("localnet")` discovery for sb-local host/port + localnet RPC + wallet.
- Build-from-source on dev; e2e test (`lwalrus store/read/blob-status`).
- MVP subset first: `store` / `read` / `blob-status`, then `store-quilt` / `read-quilt` / `delete` / `info` / `blob-id`.

**Phase 2 — `lsite` (deferred; only if Walrus-Sites dev parity is a stated goal):**
- 2a. Publish `walrus_site` on localnet (the real gate — resolve the suins/MVR dep first).
- 2b. Wire real `site-builder` + generate localnet `sites-config.yaml`; `lsite publish` headless + golden JSON-parity test.
- 2c. Stand up the real portal server (config-pointed, object-id/base36 only) + CORS/timeout/lifecycle glue.

**Phase 3 — polish/distribution:**
- Remaining Bucket B ops, friendly messages for all Bucket C, `get-wal` via exchange.
- Add `lwalrus` (and, if Phase 2 shipped, the portal/site-builder fetch) to the precompiled `localnet-tools` asset for staging/main; e2e CI incl. the parity test.

## Decision gates / open questions

- **Green-light Phase 1?** (Low risk, high value, recommended.)
- **Is local Walrus-Sites dev parity an explicit project goal?** If no → stop after Phase 1.
- **Phase 2 gate:** if the `walrus_site` suins/MVR Move dependency resolution turns ugly, reassess — don't sink multi-day Move effort.
- One-binary-with-subcommands (`lwalrus`) vs per-tool: `lwalrus` is one `walrus`-compatible binary; `lsite` stays the real site-builder (no new binary).
