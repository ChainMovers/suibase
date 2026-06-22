// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Drop-in PARITY proof against a REAL network (testnet): the exact same generic
//! `common::parity_roundtrip` body that `localnet_roundtrip.rs` runs against
//! `WalrusLocalClient` is run here against a real `walrus_sdk::WalrusNodeClient`.
//! If the mirror's signatures/types ever drift from the SDK, neither test compiles;
//! if behavior drifts, they disagree.
//!
//! ON-DEMAND + FUND-GATED. Runs only when `WALRUS_TESTNET_TEST=1` AND a walrus client
//! config is found (env `WALRUS_TESTNET_CONFIG`, else the walrus CLI default
//! `~/.config/walrus/client_config.yaml`). It needs a funded wallet (SUI for gas + WAL
//! for storage); without funds the store call errors and the test fails loudly — that
//! is the signal to top up / convert SUI->WAL. Absent config => clean SKIP.

mod common;

use std::path::PathBuf;

use walrus_sdk::config::ClientConfig;
use walrus_sdk::node_client::WalrusNodeClient;

fn config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WALRUS_TESTNET_CONFIG") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let default = dirs_home()?.join(".config/walrus/client_config.yaml");
    default.exists().then_some(default)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[tokio::test]
async fn testnet_parity() -> anyhow::Result<()> {
    if std::env::var("WALRUS_TESTNET_TEST").is_err() {
        eprintln!("SKIP: set WALRUS_TESTNET_TEST=1 (+ a funded wallet) to run the real-network parity test");
        return Ok(());
    }
    let Some(path) = config_path() else {
        eprintln!(
            "SKIP: no walrus client config (set WALRUS_TESTNET_CONFIG=/path/to/client_config.yaml \
             or create ~/.config/walrus/client_config.yaml)"
        );
        return Ok(());
    };
    eprintln!("using walrus client config: {}", path.display());

    // Real-client construction (the standard walrus path): load config -> build the
    // contract client from the wallet in the config -> attach the committees refresher.
    let (config, ctx) = ClientConfig::load_from_multi_config(&path, Some("testnet"))?;
    eprintln!("loaded config (context: {ctx:?})");
    let sui_client = config.new_contract_client_with_wallet_in_config(None).await?;
    let client = WalrusNodeClient::new_contract_client_with_refresher(config, sui_client).await?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let payload = format!("walrus-local-sdk testnet parity {nonce}").into_bytes();

    // The IDENTICAL generic body used by the localnet test.
    common::parity_roundtrip(&client, &payload).await?;

    eprintln!("testnet drop-in parity: PASS");
    Ok(())
}
