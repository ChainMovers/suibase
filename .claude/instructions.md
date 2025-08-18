# Suibase Development - Claude Code Instructions

## Local Sui Reference Repository

**ALWAYS prioritize local repository over GitHub API for Sui codebase queries.**

### Setup
- **Path**: `~/repos/sui-reference-main` (or `$SUI_REFERENCE_PATH`)
- **Initialize**: `scripts/manage-sui-reference.sh init`
- **Update**: `scripts/manage-sui-reference.sh auto`
- **Status**: `scripts/manage-sui-reference.sh status`

### Efficient Search Strategy

**PRIMARY**: Use local repository with Grep tool
```
Grep pattern:"your_pattern" path:"~/repos/sui-reference-main"
```

**SECONDARY**: Use pre-built search cache (faster)
```
Read file_path:"~/repos/sui-reference-main/.claude_search_cache/rust_files.txt"
Read file_path:"~/repos/sui-reference-main/.claude_search_cache/structs.txt"
```

**FALLBACK**: GitHub API only if local unavailable

### Performance Optimizations

1. **File Discovery**: Check cached file lists first
   - `~/repos/sui-reference-main/.claude_search_cache/rust_files.txt`
   - `~/repos/sui-reference-main/.claude_search_cache/toml_files.txt`
   - `~/repos/sui-reference-main/.claude_search_cache/doc_files.txt`

2. **Pattern Searches**: Use cached common patterns
   - `structs.txt` for struct definitions
   - `enums.txt` for enum definitions  
   - `functions.txt` for function signatures

3. **Code Analysis**: Read directly from local files
   - Faster than API calls
   - Full file context available
   - No rate limiting

### Integration with Suibase Development

- **Sui Node Understanding**: Reference `~/repos/sui-reference-main/crates/sui-node/`
- **RPC Implementation**: Check `~/repos/sui-reference-main/crates/sui-json-rpc*/`
- **Framework Changes**: Monitor `~/repos/sui-reference-main/crates/sui-framework/`
- **Protocol Updates**: Track `~/repos/sui-reference-main/crates/sui-protocol-config/`

### Automation

- Repository auto-updates via pre-session hooks
- Health checks integrated with project workflow
- Minimal maintenance required - script handles everything

### Token Efficiency

- **Reduce tokens**: Use cached search results instead of large file reads
- **Targeted queries**: Grep specific patterns rather than reading entire files
- **Smart fallbacks**: Only use GitHub when local repo unavailable
- **Pre-computed indices**: Faster responses with less computational overhead

This setup provides 10x faster Sui codebase queries while reducing Claude Code token usage and eliminating GitHub API rate limits.