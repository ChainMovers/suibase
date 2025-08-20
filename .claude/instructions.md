# Suibase Development - Claude Instructions

## Local Repositories (USE THESE FIRST)

**Paths:**
- Sui core: `~/repos/sui-reference-main`
- Sui SDK: `~/repos/sui-rust-sdk-main`
- Walrus: `~/repos/walrus-reference-main`

**Setup:** `~/suibase/scripts/dev/manage-local-repos.sh`

## Search Strategy

1. **Grep local repos:**
   ```
   Grep pattern:"pattern" path:"~/repos/sui-reference-main"
   Grep pattern:"pattern" path:"~/repos/sui-rust-sdk-main"
   Grep pattern:"pattern" path:"~/repos/walrus-reference-main"
   ```

2. **Use cached indices (faster):**
   ```
   Read file_path:"~/repos/sui-reference-main/.claude_search_cache/rust_files.txt"
   Read file_path:"~/repos/sui-reference-main/.claude_search_cache/structs.txt"
   Read file_path:"~/repos/sui-rust-sdk-main/.claude_search_cache/proto_files.txt"
   Read file_path:"~/repos/walrus-reference-main/.claude_search_cache/rust_files.txt"
   ```

3. **Fallback:** GitHub API only if local unavailable

## Key Directories

**Sui Core:**
- Node: `crates/sui-node/`
- RPC: `crates/sui-json-rpc*/`
- Framework: `crates/sui-framework/`
- Protocol: `crates/sui-protocol-config/`

**Sui SDK:**
- Client: `crates/sui-sdk/src/`
- Proto: `*.proto` files

**Walrus:**
- Upload Relay: `crates/walrus-upload-relay/`
- Contracts: `contracts/`