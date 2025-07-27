# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Testing
- **Run all tests**: `~/suibase/scripts/tests/run-all.sh`
  - Comprehensive test suite including unit tests, integration tests, and scripts validation
  - Returns exit code 0 for success, 1 for fatal errors, 2 for skipped tests
  - Test categories can be run individually with flags like `--rust-tests`, `--scripts-tests`

### Rust Development
- **Build Rust crates**: `cd rust/suibase && cargo build`
- **Run Rust tests**: `cd rust/[crate-name] && cargo test`
  - Main crates: `rust/suibase`, `rust/helper`, `rust/helper-uniffi`, `rust/demo-app`
- **Lint Rust code**: Use the project's lint script at `scripts/dev/lint`

### TypeScript/JavaScript Development  
- **Build Sui Explorer**: `cd typescript/sui-explorer && pnpm build`
- **Serve Sui Explorer**: `cd typescript/sui-explorer && pnpm serve`
- **VSCode Extension**: Located in `typescript/vscode-extension/`

### Daemon Management
- **Start daemon**: `scripts/dev/start-daemon`
- **Stop daemon**: `scripts/dev/stop-daemon`
- **Update daemon**: `scripts/dev/update-daemon`

## Architecture Overview

### Core Components

**Suibase** is a development tool for Sui blockchain that provides:
- Multi-network workdir management (localnet, devnet, testnet, mainnet)
- JSON-RPC proxy server with failover and load balancing
- Local Sui explorer interface
- Rust and Python helpers for test automation

### Directory Structure

- **`scripts/`**: Shell scripts for network management and utilities
  - `localnet`, `devnet`, `testnet`, `mainnet`: Network-specific commands
  - `common/`: Shared shell utilities and configuration
  - `dev/`: Development tools (lint, daemon management)
- **`rust/`**: Rust crates and applications
  - `suibase/`: Main daemon with workspace crates (suibase-daemon, common, poi-server)
  - `helper/`: Rust helper library for Suibase integration
  - `demo-app/`: Example Rust application using Suibase
- **`typescript/`**: TypeScript/JavaScript components
  - `sui-explorer/`: Local Sui explorer web interface
  - `vscode-extension/`: VSCode extension for Suibase
- **`workdirs/`**: Runtime environments for different networks
  - `active`: Symlink to currently active workdir
  - `localnet/`, `devnet/`, `testnet/`, `mainnet/`: Network-specific configurations
  - `common/`: Shared data and logs
- **`python/`**: Python SDK and demos
- **`move/`**: Move language packages for Suibase

### Workdir System

Suibase uses a "workdir" concept where each network (localnet, devnet, testnet, mainnet) has its own isolated environment with:
- Sui client configuration (`config/client.yaml`)
- Keystores and aliases (`sui.keystore`, `sui.aliases`)
- Network-specific binaries and repositories
- Individual `suibase.yaml` configuration files

The `workdirs/active` symlink points to the currently active workdir.

### Daemon Architecture

The Rust daemon (`suibase-daemon`) uses:
- Tokio async runtime with graceful shutdown support
- Arc<RwLock> for thread-safe shared state
- Event-driven architecture with periodic audit for consistency
- Three message types: EVENT_AUDIT (read-only checks), EVENT_UPDATE (state changes), EVENT_EXEC (command execution)

### Key Scripts

- **Network commands**: `localnet`, `devnet`, `testnet`, `mainnet` - Start/stop/status for each network
- **Sui wrapper**: `sui` - Context-aware Sui CLI that uses active workdir
- **Update system**: `update` - Updates Suibase and Sui binaries
- **Workdir management**: Scripts automatically handle workdir switching and configuration

## Development Workflow

1. **Environment Setup**: Suibase manages Sui installations automatically, no manual compilation needed
2. **Network Testing**: Use `localnet start` for local development, other networks for integration testing  
3. **Configuration**: Each workdir has its own `suibase.yaml` for network-specific settings
4. **Debugging**: Logs are centralized in `workdirs/common/logs/`
5. **Testing**: Always run the full test suite before submitting changes

## Important Notes

- Suibase can coexist with other Sui installations safely
- Uses official Mysten Labs binaries when available, falls back to source compilation
- All daemon processes support auto-restart on panic for reliability
- The system is designed for "eventual consistency" with periodic audits