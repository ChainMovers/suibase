# CLAUDE.md

AI agent guidance for Suibase development.

## Architecture

**Suibase**: Sui blockchain dev tool with multi-network workdir management, JSON-RPC proxy, explorer UI.

**Workdir System**: Isolated environments per network with `config/client.yaml`, keystores, `suibase.yaml` configs.

## Commands

**Tests**: `~/suibase/scripts/tests/run-all.sh` (destructive, requires permission)
**Rust**: `cd rust/suibase && cargo build/test`
**TS**: `cd typescript/sui-explorer && pnpm build/serve`
**suibase-daemon lifecycle**: Devs-only commands are `scripts/dev/{start,stop,update}-daemon`
**localnet lifecycle**: Workdir user commands localnet {start, stop, update, regen}

## Design

**Structure**:
- `scripts/`:
    - User workdir commands {`localnet`, `devnet`, `testnet`, `mainnet`} {update, start, stop}
    - {dsui,tsui,lsui,msui} as 'sui' client wrapper that are workdir aware.
- `rust/suibase/crates/suibase-daemon/`: Singleton suibase-daemon, to run all services.
- `typescript/`: Explorer UI, VSCode extension
- `workdirs/`: Per-network environments (configs, keystores, binaries)
  - `active`: symlink to current workdir

**suibase-daemon**: See rust/suibase/crates/suibase-daemon/CLAUDE.md

**Notes**: Coexists with other Sui installs, uses official binaries + fallback compilation, auto-restart on panic.