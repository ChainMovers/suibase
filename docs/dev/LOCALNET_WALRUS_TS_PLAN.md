# LOCALNET_WALRUS_TS_PLAN

> **STATUS: DONE (2026-06-21).** Implemented as `typescript/walrus-local-sdk`
> (`@suibase/walrus-local`). See `docs/dev/LOCALNET_WALRUS_FEATURE.md` →
> [TypeScript client](LOCALNET_WALRUS_FEATURE.md#typescript-client) and the package README.
> NB: the design below evolved during implementation — the final approach is **NOT** the
> original "thin HTTP client" but a **subclass of the real `@mysten/walrus` `WalrusClient`**.
>
> **Final architecture (supersedes the plan below).** `class WalrusLocalClient extends
> WalrusClient`, constructed with `super({ packageConfig, suiClient })` against the localnet
> deploy. Targets the **latest `@mysten/walrus` (1.2.x)** so signatures + the `WalrusFile`
> quilt API match exactly. **On-chain methods are inherited unchanged** (delete/extend/
> attributes/storageCost/systemState/…); only **node-talking** methods are overridden to use
> sb-local (`readBlob`/`writeBlob`/`writeFiles`/`writeQuilt`/`getFiles`/`getBlob`/
> `getVerifiedBlobStatus`); inherently node-only plumbing throws `UNSUPPORTED`.
>
> **Decisions made (by the user, during build):** extend `@mysten/walrus` (not hand-roll);
> target latest (quilts via `WalrusFile`); match Mysten's constructor (`new WalrusLocalClient()`,
> **no `forWorkdir`**); name `@suibase/walrus-local`. Deps: `@mysten/walrus` + `@mysten/sui`
> (peer). **Node ≥ 22.** Two sb-local additions: a `deletable=true` PUT param + a localnet-only
> `GET /v1/blobs/{id}/status` route.
>
> **Delivered + validated live (Node 22):** the full developer surface — writeBlob/readBlob,
> deleteBlob/extendBlob/attributes/storageCost (inherited, on-chain), writeQuilt/writeFiles/
> getFiles/getBlob (via sb-local), getVerifiedBlobStatus. Tests: 9 unit (errors + config) +
> 11 live integration (gated; cross-env blob_id fixture proven from TS), 20/20 passing. The
> remainder of this file is the **original** plan, kept for historical context only.

---

Short plan to bring the localnet Walrus experience to **TypeScript**, mirroring what
`rust/walrus-local-sdk` did for Rust. Resume in a fresh session (this one ran out of context).

## Where we are (already done, Rust side)

- **`sb-local`** is a running, wire-compatible Walrus **aggregator + publisher HTTP API** on
  `http://localhost:45840` (localnet). Its wire contract — including error codes — is
  verified equal to the real Mysten testnet aggregator/publisher
  (`scripts/tests/050_walrus_tests/test_sb_local_wire_parity.sh`).
- **Cross-environment `blob_id` parity is proven**: the same content yields the same id on
  localnet and the real testnet publisher (n_shards=1000, walrus-core encoder).
- `rust/walrus-local-sdk` = `WalrusLocalClient`, a drop-in mirror of Mysten's `walrus_sdk`.
- Docs: `docs/src/walrus.md` has the Localnet section; `docs/dev/LOCALNET_WALRUS_FEATURE.md`.

The TS effort is **lighter than Rust**: sb-local already speaks the wire protocol, so most TS
clients just need to point at `localhost:45840`. The Rust mirror existed because the Rust SDK
is on-chain/node-talking; in TS the HTTP path carries most of the value.

## Goal

A TypeScript developer (and AI agents) can read/write Walrus blobs + quilts on localnet with
the same code shape they'd use on testnet/mainnet — no storage nodes, no funds, no internet.

## First investigation step (do this first)

Confirm the **current `@mysten/walrus` TS SDK** surface (npm; training may be stale):
1. Does it expose an **aggregator/publisher HTTP** client (URL-based)? If yes, it works
   against sb-local by changing only the base URL → the TS story may be mostly **config + docs
   + a parity test**, not a new SDK.
2. Its main `WalrusClient` is **node-talking** (reads/writes slivers from storage nodes). That
   path **cannot** work on localnet — document it as unsupported on localnet; localnet
   uses the HTTP path.
3. Check `@mysten/walrus` quilt + blob-status + by-object-id APIs and their HTTP equivalents
   (sb-local already serves quilts, by-object-id, by-quilt-patch-id, /v1/quilts).

## Recommended approach (decide after step 1)

- **Plan A (preferred, thin):** a small TS package — e.g. `typescript/walrus-local-sdk`
  (or extend `typescript/helper`) — exposing a `WalrusLocalClient` over sb-local's HTTP API:
  `store/storeQuilt/read/readByObjectId/readQuiltPatch/getQuiltPatches/status`, returning the
  same JSON shapes sb-local emits (which mirror the real daemon's `BlobStoreResult` etc.).
  Tiny dep (just `fetch`). Optionally a `for_workdir("localnet")` that reads the port from
  `suibase.yaml` defaults (45840).
- **Plan B (only if @mysten/walrus has a clean HTTP client):** thin wrapper / docs showing how
  to point `@mysten/walrus` at sb-local — minimal code, maximal reuse.
- **Out of scope:** mirroring the node-talking `WalrusClient` off-node (the Rust mirror wrapped
  an on-chain client; the TS node-client is sliver/encoding-heavy and not worth off-node-faking).

## Milestones

- **M0** Recon `@mysten/walrus` API + decide A vs B (above).
- **M1** Minimal TS client over sb-local HTTP (store/read), with a `localnet` constructor.
- **M2** Quilts + by-object-id + status to match the Rust/sb-local surface.
- **M3** Tests (vitest, like `typescript/helper`): localnet round-trip (gated on sb-local up)
  + a **parity test** that runs the same body against the real Mysten testnet
  aggregator/publisher (fund-free for reads/errors; reuse the wire-parity idea). Assert the
  cross-env `blob_id` equality (the proven fixture id: `x37bth2QxQZBbjZS6F-6l9mU_-bp46CRfOo33IAwe2U`
  for content `"walrus-local-sdk cross-environment blob_id fixture v1"`).
- **M4** Docs: add a TS example to `docs/src/walrus.md` Localnet section; CI wire-up.

## Open questions / decisions for the user

- Package home: new `typescript/walrus-local-sdk` vs a module inside `typescript/helper`?
- Publish to npm (`@suibase/...`) or repo-internal only? (resolved: name `@suibase/walrus-local`)
- Should the TS client also expose pools (engine-only in Rust, not in the HTTP API today —
  sb-local would need pool routes first)?
- How much to lean on `@mysten/walrus` types vs hand-rolled TS types for drop-in feel.

## Reusable assets

- `sb-local` HTTP API + `test_sb_local_wire_parity.sh` (wire/error parity vs Mysten).
- `scripts/dev/run-tests-with-testnet` pattern (skip-is-failure, real-testnet gating).
- The Mysten public testnet endpoints used for parity:
  aggregator `https://aggregator.walrus-testnet.walrus.space`,
  publisher `https://publisher.walrus-testnet.walrus.space` (publisher free on testnet).
- Docs structure already established in `docs/src/walrus.md`.

## Autonomy verdict

**Yes, largely autonomous**, and smaller than the Rust effort — the heavy lifting (self-contained
deploy, certify, wire-compatible HTTP, blob_id parity) is done. The main unknowns are the
current `@mysten/walrus` API shape and the package-placement decision (resolve M0 + the open
questions early). TS test infra exists (`typescript/helper` uses pnpm/vitest).
