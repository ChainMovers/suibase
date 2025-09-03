# Walrus SDK Tests

TypeScript tests for Walrus SDK + Suibase upload relay integration.

## Requirements
- `CI_WORKDIR="testnet"` (uses active address from Suibase keystore)
- Node.js 18.0.0+, testnet workdir only

## Execution
```bash
CI_WORKDIR="testnet" ~/suibase/scripts/tests/060_walrus_sdk_tests/upload-test.sh
```

## Exit Codes
- `0`: Success, `1`: Failed, `2`: Skipped

## Notes
- Uses active address from Suibase keystore automatically
- Gracefully skips if insufficient SUI/WAL balance (< 0.05) or active address unavailable
- `__test_common.sh` handles setup/build automatically