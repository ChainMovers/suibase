# Suibase Helper for TypeScript

API to Suibase, intended for development of Sui tool/test automation (Playwright, Vitest, etc.)
and TypeScript-based backends.

Functionally equivalent to the [Rust helper](../../rust/helper/).

## Install

This package is not published to npm. Reference it via a local path so the helper and your
Suibase installation are guaranteed to match versions.

```jsonc
// In your project's package.json
{
  "devDependencies": {
    "suibase": "file:../../suibase/typescript/helper"
  }
}
```

Adjust the relative path depending on where your `package.json` is located relative to
`~/suibase`. The helper has **zero runtime dependencies**.

## Usage

```ts
import { Helper } from "suibase";

const sbh = new Helper();
if (sbh.isInstalled()) {
  sbh.selectWorkdir("localnet");
  console.log("active address is", sbh.clientAddress("active"));
  console.log("demo package id is", sbh.packageId("demo"));
}
```

## API

| Method | Description |
| --- | --- |
| `isInstalled()` | Returns `true` if `~/suibase` is found. |
| `selectWorkdir(name)` | Pick one of `active`, `localnet`, `devnet`, `testnet`, `mainnet`, … |
| `workdir()` | Name of the currently selected workdir (resolves `active`). |
| `keystorePathname()` | Absolute path to `sui.keystore`. |
| `packageId(name)` | Object ID of the last successfully published Move package. |
| `publishedNewObjects(type)` | Object IDs of objects created when the package was published. `type` format: `package::module::type`. |
| `clientAddress(name)` | Sui address by name (e.g. `active`, `sb-1-ed25519`, …). |
| `rpcUrl()` | JSON-RPC URL for the selected workdir. |
| `wsUrl()` | WebSocket URL for the selected workdir. |

All methods are synchronous. Failures throw `SuibaseError` with a `code` field.

## Self-contained

This helper is **not** part of the Suibase installation. End-users of Suibase do not need
Node.js installed. Only consumers of this helper need Node, when they build their own project.
