// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! Nodeless localnet Walrus deploy (Layer A) for Suibase.
//!
//! Publishes the Walrus Move packages to the *running* localnet Sui, sets up an
//! N=1 deterministic committee whose BLS key we hold (for off-node `certify_blob`),
//! creates + funds a SUI->WAL exchange, and writes:
//!   - <out-config>     walrus CLI config (contexts: ids + rpc + wallet)
//!   - <out-descriptor> suibase descriptor (package id + held committee key + chain id)
//!
//! NO storage nodes are started (nodeless). Real Blob/Storage objects + held-key
//! certify happen on the localnet Sui; bytes are served from the filesystem by the
//! WalrusStore client. See docs/dev/LOCALNET_WALRUS_PLAN.md.

use std::{
    num::NonZeroU16,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use walrus_core::keys::{NetworkKeyPair, ProtocolKeyPair};
use walrus_sui::{
    client::{ReadClient, SuiContractClient},
    config::load_wallet_context_from_path,
    test_utils::system_setup::{
        create_and_init_system_for_test, end_epoch_zero, register_committee_and_stake,
        SystemContext,
    },
    types::NodeRegistrationParams,
};

/// 1 WAL in FROST.
const ONE_WAL: u64 = 1_000_000_000;
/// WAL minted into the exchange so the mock can convert SUI -> WAL to pay for storage.
const EXCHANGE_FUND_WAL: u64 = 1_000_000 * ONE_WAL;

/// The Walrus Move contracts, vendored from the git-pinned walrus rev and embedded
/// into the binary so a precompiled artifact needs no walrus checkout at runtime.
/// Kept in sync with the pinned rev via embedded-contracts/CONTRACTS.sha256 (drift-guard).
static EMBEDDED_CONTRACTS: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/embedded-contracts");

/// Extract the embedded contracts under `<deploy_dir>/contracts-src` and return that
/// directory (the parent that contains the wal/wal_exchange/walrus/walrus_subsidies
/// package dirs, so their `../wal`-style local deps resolve).
fn materialize_embedded_contracts(deploy_dir: &Path) -> Result<PathBuf> {
    let out = deploy_dir.join("contracts-src");
    if out.exists() {
        std::fs::remove_dir_all(&out).ok();
    }
    std::fs::create_dir_all(&out)?;
    EMBEDDED_CONTRACTS
        .extract(&out)
        .context("extracting embedded contracts")?;
    Ok(out)
}

#[derive(Parser)]
#[command(name = "walrus-localnet-deploy", about = "Nodeless localnet Walrus deploy for Suibase")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Publish + set up an N=1 committee off-node + write config/descriptor.
    Deploy(DeployArgs),
}

#[derive(clap::Args)]
struct DeployArgs {
    /// Direct localnet Sui JSON-RPC URL (publish target).
    #[arg(long)]
    rpc: String,
    /// Localnet faucet URL (kept for parity; unused while the wallet is pre-funded).
    #[arg(long)]
    faucet: Option<String>,
    /// Suibase localnet wallet config (client.yaml) — keystore + addresses.
    #[arg(long)]
    wallet: PathBuf,
    /// Walrus Move contracts dir. Defaults to the git-pinned checkout's contracts/.
    #[arg(long)]
    contracts: Option<PathBuf>,
    /// Where to write the walrus CLI config (contexts / ids / rpc / wallet).
    #[arg(long)]
    out_config: PathBuf,
    /// Where to write the suibase descriptor (package id + held committee key + chain id).
    #[arg(long)]
    out_descriptor: PathBuf,
    /// Total shards (all assigned to the single committee member).
    #[arg(long, default_value_t = 1000)]
    n_shards: u16,
    /// Live chain id, recorded in the descriptor (the caller uses it for idempotency).
    #[arg(long)]
    chain_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Deploy(args) => deploy(args).await,
    }
}

async fn deploy(args: DeployArgs) -> Result<()> {
    let n_shards = NonZeroU16::new(args.n_shards).context("n_shards must be > 0")?;
    let deploy_dir = args
        .out_config
        .parent()
        .map(|p| p.join("walrus-localnet-deploy-tmp"))
        .unwrap_or_else(|| PathBuf::from("/tmp/walrus-localnet-deploy-tmp"));
    std::fs::create_dir_all(&deploy_dir)?;

    // The Suibase localnet wallet defaults to the proxy env; publish via the direct
    // fullnode RPC instead. Write a sibling wallet config pinned to --rpc.
    let wallet_path = direct_rpc_wallet(&args.wallet, &args.rpc, &deploy_dir)
        .context("preparing direct-rpc wallet")?;
    let admin_wallet =
        load_wallet_context_from_path(Some(&wallet_path), None).context("loading admin wallet")?;

    // Hold the N=1 committee BLS key in-process; persist it to the descriptor.
    let committee_key = ProtocolKeyPair::generate();

    // contracts=None -> the vendored contracts embedded in this binary, so a
    // precompiled artifact is self-contained at runtime (matched to the git-pinned
    // walrus rev via embedded-contracts/CONTRACTS.sha256).
    let contract_dir = match args.contracts {
        Some(d) => d,
        None => materialize_embedded_contracts(&deploy_dir)
            .context("materializing embedded walrus contracts")?,
    };

    let (sysctx, client) = create_and_init_system_for_test(
        admin_wallet,
        n_shards,
        Duration::from_secs(0),    // epoch_zero_duration
        Duration::from_secs(3600), // epoch_duration (1h: epoch stable during dev)
        None,                      // max_epochs_ahead (default)
        false,                     // with_credits
        Some(deploy_dir.clone()),
        Some(contract_dir),
    )
    .await
    .context("publishing + initializing the Walrus system")?;

    // Register + stake the single node off-node, then end epoch 0.
    let net_key = NetworkKeyPair::generate();
    let node_params = vec![NodeRegistrationParams::new_for_test(
        committee_key.public(),
        net_key.public(),
    )];
    let bls_keys = vec![committee_key.clone()];
    let node_clients = vec![&client];
    register_committee_and_stake(
        &client,
        &node_params,
        &bls_keys,
        &node_clients,
        &[ONE_WAL],
        Some(1),
    )
    .await
    .context("registering + staking the committee")?;
    end_epoch_zero(&client).await.context("ending epoch 0")?;

    // Poll until the committee is live (localnet has read-after-write lag).
    let mut committee = client.read_client().current_committee().await?;
    for _ in 0..15 {
        if !committee.members().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        committee = client.read_client().current_committee().await?;
    }
    if committee.members().is_empty() {
        bail!("committee did not become live after staking + end_epoch_zero");
    }

    // Create + fund a SUI->WAL exchange the mock can use to pay for storage.
    let exchange_id: Option<String> = match sysctx.wal_exchange_pkg_id {
        Some(pkg) => Some(
            client
                .create_and_fund_exchange(pkg, EXCHANGE_FUND_WAL)
                .await
                .context("creating + funding WAL exchange")?
                .to_string(),
        ),
        None => None,
    };

    write_walrus_config(&args.out_config, &sysctx, exchange_id.as_deref(), &args.rpc, &args.wallet)
        .context("writing walrus-config.yaml")?;
    write_descriptor(
        &args.out_descriptor,
        &sysctx,
        &committee_key,
        exchange_id.as_deref(),
        args.chain_id.as_deref(),
        committee.epoch,
    )
    .context("writing suibase descriptor")?;

    println!(
        "walrus-localnet-deploy: OK (epoch={} members={} n_shards={} pkg={} system={})",
        committee.epoch,
        committee.members().len(),
        sysctx.n_shards,
        sysctx.walrus_pkg_id,
        sysctx.system_object
    );
    Ok(())
}

/// Suibase's localnet client.yaml defaults to the proxy env; produce a sibling
/// wallet config whose active env points directly at `rpc`.
fn direct_rpc_wallet(src: &Path, rpc: &str, deploy_dir: &Path) -> Result<PathBuf> {
    let raw =
        std::fs::read_to_string(src).with_context(|| format!("reading {}", src.display()))?;
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&raw)?;

    let alias = doc
        .get("envs")
        .and_then(|e| e.as_sequence())
        .into_iter()
        .flatten()
        .find(|env| env.get("rpc").and_then(|r| r.as_str()) == Some(rpc))
        .and_then(|env| env.get("alias").and_then(|a| a.as_str()))
        .map(|s| s.to_string())
        .with_context(|| format!("no env in {} has rpc {}", src.display(), rpc))?;

    doc["active_env"] = serde_yaml::Value::String(alias);
    let out = deploy_dir.join("admin_wallet.yaml");
    std::fs::write(&out, serde_yaml::to_string(&doc)?)?;
    Ok(out)
}

fn write_walrus_config(
    path: &Path,
    sysctx: &SystemContext,
    exchange_id: Option<&str>,
    rpc: &str,
    wallet: &Path,
) -> Result<()> {
    let exchange_block = match exchange_id {
        Some(id) => format!("    exchange_objects:\n      - {id}\n"),
        None => String::new(),
    };
    let yaml = format!(
        "# Suibase localnet Walrus config (NODELESS). Ephemeral: rewritten on each\n\
         # 'localnet start'/'regen' by deploy_walrus_localnet(). Do not edit by hand.\n\
         contexts:\n\
         \x20 localnet:\n\
         \x20   system_object: {system}\n\
         \x20   staking_object: {staking}\n\
         {exchange_block}\
         \x20   wallet_config:\n\
         \x20     path: {wallet}\n\
         \x20     active_env: localnet\n\
         \x20   rpc_urls:\n\
         \x20     - {rpc}\n\
         default_context: localnet\n",
        system = sysctx.system_object,
        staking = sysctx.staking_object,
        exchange_block = exchange_block,
        wallet = wallet.display(),
        rpc = rpc,
    );
    std::fs::write(path, yaml)?;
    Ok(())
}

fn write_descriptor(
    path: &Path,
    sysctx: &SystemContext,
    committee_key: &ProtocolKeyPair,
    exchange_id: Option<&str>,
    chain_id: Option<&str>,
    epoch: u32,
) -> Result<()> {
    let yaml = format!(
        "# Suibase nodeless localnet Walrus descriptor (ephemeral; rewritten each regen).\n\
         # Consumed by the WalrusStore localnet mock. NOT a walrus CLI file.\n\
         chain_id: {chain}\n\
         epoch: {epoch}\n\
         package_id: {pkg}\n\
         system_object: {system}\n\
         staking_object: {staking}\n\
         wal_exchange_pkg_id: {wal_pkg}\n\
         exchange_object: {exchange}\n\
         treasury_object: {treasury}\n\
         n_shards: {n_shards}\n\
         # Held committee BLS keypair (flag||scalar, Base64). LOCALNET ONLY.\n\
         committee_protocol_keypair: {key}\n",
        chain = chain_id.unwrap_or("unknown"),
        epoch = epoch,
        pkg = sysctx.walrus_pkg_id,
        system = sysctx.system_object,
        staking = sysctx.staking_object,
        wal_pkg = opt(sysctx.wal_exchange_pkg_id.as_ref()),
        exchange = opt(exchange_id),
        treasury = opt(sysctx.treasury_object.as_ref()),
        n_shards = sysctx.n_shards,
        key = committee_key.to_base64(),
    );
    std::fs::write(path, yaml)?;
    Ok(())
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "null".to_string())
}
