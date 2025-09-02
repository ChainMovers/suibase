# Walrus SDK Tests

TypeScript tests for Walrus SDK + Suibase upload relay integration.

## Requirements
- `CI_WORKDIR="testnet"` and `SECRET_TESTNET_ACCOUNT="dummy_key_or_mnemonic"`
- Node.js 18.0.0+, testnet workdir only

## Execution
```bash
CI_WORKDIR="testnet" SECRET_TESTNET_ACCOUNT="dummy_key_or_mnemonic" ~/suibase/scripts/tests/060_walrus_sdk_tests/test_walrus_sdk_upload.sh
```

## Exit Codes
- `0`: Success, `1`: Failed, `2`: Skipped

## Notes
- `__test_common.sh` handles setup/build automatically
- `dist/` auto-generated (gitignored)