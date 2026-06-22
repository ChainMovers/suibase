// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

//! `lwalrus` — a localnet-only, `walrus`-CLI-shaped frontend over the localnet
//! Walrus engine ([`WalrusLocalClient`] / the same store `sb-local` serves).
//!
//! WHY THIS EXISTS: the real Mysten `walrus` CLI talks DIRECTLY to storage nodes
//! (its `ClientConfig` has no aggregator/publisher backend mode), and the suibase
//! localnet runs no storage nodes. So the real binary cannot be merely
//! config-pointed at localnet the way `twalrus`/`mwalrus` point at the real
//! networks. `lwalrus` dispatches the storage commands to the already-built
//! `WalrusLocalClient` (rust/walrus-local-sdk), a drop-in mirror of `walrus_sdk`.
//!
//! PARITY MODEL (see docs/dev/LWALRUS_LSITE_PLAN.md): lwalrus is a SUBSET of the
//! `walrus` CLI. It is NOT a byte-exact `--help` clone (different command set,
//! and the localnet stack is pinned to a different walrus rev than the shipped
//! binary). Instead it maintains an explicit "Not supported for localnet:" list
//! (see NOT_SUPPORTED_HELP + the external-subcommand handler): invoking anything
//! unsupported prints a clear "Not supported for localnet" message, and the
//! parity test (scripts/tests/050_walrus_tests/test_lwalrus_parity.sh) compares
//! only the SUPPORTED surface against the real `walrus`, ignoring this list.

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use walrus_core::BlobId;
use walrus_local_sdk::WalrusLocalClient;
use walrus_sdk::node_client::store_args::StoreArgs;

/// Appended to `lwalrus --help`. Enumerates the `walrus` commands lwalrus does
/// NOT implement on the localnet, grouped by reason. This is the single
/// source of truth the parity test reads to know what to ignore — when `walrus`
/// adds a command that is neither supported here nor listed below, the test
/// fails so we triage it (support it, or add it here). Keep in sync with `walrus`.
const NOT_SUPPORTED_HELP: &str = "\
Not supported for localnet (commands):
  Daemon/HTTP (suibase already serves these, and wal-relay covers the relay):
    aggregator, publisher, daemon
  Staking & committee (the localnet uses a held-key N=1 committee):
    stake, request-withdraw-stake, withdraw-stake, list-staked-wal
  Storage-node operator (there are no storage nodes on a localnet):
    node-admin, health, pull-archive-blobs, blob-backfill
  Not yet implemented in this MVP (planned):
    store-quilt, read-quilt, list-patches-in-quilt, info, blob-id, convert-blob-id,
    list-blobs, extend, share, burn-blobs, fund-shared-blob, get-wal,
    get-blob-attribute, set-blob-attribute, remove-blob-attribute,
    remove-blob-attribute-fields, generate-sui-wallet, json

Not supported for localnet (options):
  The binary, config, context, wallet and RPC are implicit (always the running
  localnet), so these `walrus` global options are rejected:
    --config, --context, --wallet, --rpc-url, --gas-budget, --trace-cli
  No storage nodes / tip economy on a localnet, so these are rejected:
    --dry-run, --force, --ignore-resources, --share, --end-epoch,
    --earliest-expiry-time, --upload-relay, --skip-tip-confirmation,
    --child-process-uploads, --timeout, --strict-consistency-check,
    --skip-consistency-check, --no-status-check, --yes
  (Supported: --epochs/--permanent on `store`, --out on `read`, and --json.)

Invoking an unsupported command or global option prints \"Not supported for localnet\".";

#[derive(Parser)]
#[command(
    name = "lwalrus",
    about = "Localnet Walrus client",
    version,
    after_help = NOT_SUPPORTED_HELP,
    after_long_help = NOT_SUPPORTED_HELP
)]
struct Cli {
    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long, global = true)]
    json: bool,

    // Unsupported `walrus` global options. Accepted-but-rejected and HIDDEN from help
    // (documented in NOT_SUPPORTED_HELP's options section): on the localnet the
    // binary/config/context/wallet/RPC are implicit, so passing one of these prints a
    // clear "Not supported for localnet: --X" instead of clap's "unexpected argument".
    #[arg(long, global = true, hide = true)]
    config: Option<String>,
    #[arg(long, global = true, hide = true)]
    context: Option<String>,
    #[arg(long, global = true, hide = true)]
    wallet: Option<String>,
    #[arg(long = "rpc-url", global = true, hide = true)]
    rpc_url: Option<String>,
    #[arg(long = "gas-budget", global = true, hide = true)]
    gas_budget: Option<String>,
    #[arg(long = "trace-cli", global = true, hide = true)]
    trace_cli: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// Store a file as a blob on the localnet Walrus.
    Store {
        /// File to store.
        file: PathBuf,
        /// Number of epochs to store for.
        #[arg(long, default_value_t = 1)]
        epochs: u32,
        /// Store as permanent (cannot be deleted before expiry). Default is deletable.
        #[arg(long)]
        permanent: bool,
    },
    /// Read a blob by id; writes to --out, or to stdout if omitted.
    Read {
        /// Blob id (URL-safe base64, as printed by `store`).
        #[arg(value_parser = parse_blob_id)]
        blob_id: BlobId,
        /// Output file. Default: stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Print the status of a blob (Permanent / Deletable / Nonexistent ...).
    BlobStatus {
        /// Blob id (URL-safe base64).
        #[arg(value_parser = parse_blob_id)]
        blob_id: BlobId,
    },
    /// Delete an owned, *deletable* blob by id (no-op for permanent/absent blobs).
    Delete {
        /// Blob id (URL-safe base64).
        #[arg(value_parser = parse_blob_id)]
        blob_id: BlobId,
    },
    /// Any other `walrus` (sub)command — not supported on the localnet.
    #[command(external_subcommand)]
    NotSupported(Vec<String>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Reject unsupported global options with the same clear message as unsupported
    // commands (these are HIDDEN in clap, so without this they'd error as "unexpected
    // argument"). Checked before opening the engine — no running localnet needed.
    for (name, present) in [
        ("--config", cli.config.is_some()),
        ("--context", cli.context.is_some()),
        ("--wallet", cli.wallet.is_some()),
        ("--rpc-url", cli.rpc_url.is_some()),
        ("--gas-budget", cli.gas_budget.is_some()),
        ("--trace-cli", cli.trace_cli.is_some()),
    ] {
        if present {
            eprintln!("Not supported for localnet: {name}");
            eprintln!("On localnet the binary/config/context/wallet/RPC are implicit.");
            std::process::exit(1);
        }
    }

    // Handle the "unsupported command" path BEFORE opening the engine, so the
    // message is instant and does not depend on a running localnet.
    if let Cmd::NotSupported(args) = &cli.cmd {
        let name = args.first().map(String::as_str).unwrap_or("<command>");
        eprintln!("Not supported for localnet: {name}");
        eprintln!("Run 'lwalrus --help' for the supported commands.");
        std::process::exit(1);
    }

    let client = WalrusLocalClient::for_workdir("localnet").await.map_err(|e| {
        anyhow!(
            "failed to open the localnet Walrus engine ({e}).\n\
             Is the localnet started with 'walrus_local_enabled: true' (then 'localnet regen')?"
        )
    })?;

    match cli.cmd {
        Cmd::Store {
            file,
            epochs,
            permanent,
        } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let args = StoreArgs::default_with_epochs(epochs);
            let args = if permanent {
                args.permanent()
            } else {
                args.deletable()
            };
            let results = client
                .reserve_and_store_blobs(vec![bytes], &args)
                .await
                .map_err(|e| anyhow!("store failed: {e}"))?;
            let res = results
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("store returned no result"))?;
            let blob_id = res
                .blob_id()
                .ok_or_else(|| anyhow!("store produced no blob id (invalid/error result)"))?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "blobId": blob_id.to_string(),
                        "endEpoch": res.end_epoch().map(|e| e.to_string()),
                        "persistence": if permanent { "permanent" } else { "deletable" },
                    })
                );
            } else {
                println!("Stored blob.");
                println!("  blob id:     {blob_id}");
                if let Some(e) = res.end_epoch() {
                    println!("  end epoch:   {e}");
                }
                println!(
                    "  persistence: {}",
                    if permanent { "permanent" } else { "deletable" }
                );
            }
        }

        Cmd::Read { blob_id, out } => {
            let bytes = client
                .read_blob_primary(&blob_id)
                .await
                .map_err(|e| anyhow!("read failed: {e}"))?;
            match out {
                Some(path) => {
                    std::fs::write(&path, &bytes)
                        .with_context(|| format!("writing {}", path.display()))?;
                    eprintln!("Wrote {} bytes to {}", bytes.len(), path.display());
                }
                None => {
                    use std::io::Write;
                    std::io::stdout()
                        .write_all(&bytes)
                        .context("writing blob bytes to stdout")?;
                }
            }
        }

        Cmd::BlobStatus { blob_id } => {
            let status = client
                .blob_status(&blob_id)
                .await
                .map_err(|e| anyhow!("blob-status failed: {e}"))?;
            if cli.json {
                // BlobStatus's Serialize bound is not relied upon here; emit its
                // Debug form so the command is stable regardless of SDK changes.
                println!("{}", serde_json::json!({ "status": format!("{status:?}") }));
            } else {
                println!("{status:?}");
            }
        }

        Cmd::Delete { blob_id } => {
            let n = client
                .delete_owned_blob(&blob_id)
                .await
                .map_err(|e| anyhow!("delete failed: {e}"))?;
            if n > 0 {
                println!("Deleted blob {blob_id}.");
            } else {
                println!("Nothing deleted: {blob_id} is not a present, deletable, owned blob.");
            }
        }

        Cmd::NotSupported(_) => unreachable!("handled above"),
    }

    Ok(())
}

/// clap value parser for a blob id. Mirrors the real `walrus` CLI, which rejects
/// a malformed blob id at the PARSE layer (exit code 2) with the message
/// "the provided blob ID is invalid" — so lwalrus's failure matches walrus's.
fn parse_blob_id(s: &str) -> std::result::Result<BlobId, String> {
    BlobId::from_str(s).map_err(|_| "the provided blob ID is invalid".to_string())
}
