// TODO WIP Re-enable dead_code warnings after initial development writing completed.
#![allow(dead_code)]

use acoins_monitor::ACoinsMonitor;
// main.rs does:
//  - Validate command line.
//  - Telemetry setup
//  - Top level tasks started here. These runs until the program terminates:
//     - AdminController: The "leader" thread validating and applying the config changes and user actions.
//     - NetworkMonitor: Maintains all remote server stats. Info coming from multiple sources (on a mpsc channel).
//     - APIServer: Does "sandboxing" of the JSON-RPC server (auto-restart in case of panic).
//     - ClockTrigger: Send periodic audit events to other threads.
//
// Other tasks (not started here):
//
//  - ProxyServer:        One long-running thread instance per listening port. Async handling of all user traffic.
//                        Uses axum::Server and reqwest::Client. Started/stopped by the AdminController.
//
//  - RequestWorker:      Perform on-demand health check+latency test of RPC servers.
//                        Uses reqwest::Client. Started/stopped by the NetworkMonitor.
//
//  - WorkdirsWatcher:    Watch for changes to config files in the suibase workdirs. Send events to AdminController.
//                        Started/stopped by the AdminController.
//
//  - EventsWriterWorker: Manage connection(s) to subscribe/receive/dedup Sui events.
//                        One instance per workdir. Started/stopped by the AdminController.
//                        Does "sandboxing" of WebSocketWorker and DBWorker.
//
//  - WebSocketWorker:    Manage subscribe/unsubscribe and receiving Sui events for a single connection. Forwards
//                        subscribed sui events to its parent EventsWriterWorker for dedup. Uses tokio-tungstenite.
//
//  - DBWorker:           Manage the in-memory (or file) DB for a single workdir. Write to DB the already
//                        validated and dedup Sui events from its parent (EventsWriterWorker).
//
//  - CliPoller:          Periodic/on-demand CLI "status" command. Parse CLI output, update globals and generate
//                        events on changes. Sandboxed in case of parsing error (auto-restart on panic).
//                        Started by the AdminController.
//
//  - ShellWorker:        Perform external call to Suibase command line. One instance per workdir (by design, will
//                        serialize all command execution). Started/stopped by the AdminController.
//
use anyhow::Result;

use api::APIServerParams;
use clap::*;

use clock_trigger::{ClockTrigger, ClockTriggerParams};
use colored::Colorize;
use common::basic_types::MPSC_Q_SIZE;
use env_logger::{Builder, Env};

mod admin_controller;
mod api;
mod app_error;

mod acoins_monitor;
mod clock_trigger;
mod network_monitor;
mod proxy_server;
mod shared_types;
mod workdirs_watcher;
mod workers;

use shared_types::Globals;
use tokio::time::Duration;

use crate::admin_controller::AdminController;
use crate::api::APIServer;
use crate::network_monitor::NetworkMonitor;
use tokio_graceful_shutdown::{
    errors::{GracefulShutdownError, SubsystemError},
    SubsystemBuilder, Toplevel,
};

use workers::WebserverParams;
use workers::WebserverWorker;

#[allow(clippy::large_enum_variant)]
#[derive(Parser)]
#[clap(
    name = "suibase-daemon",
    about = "RPC proxy for more reliable access to Sui networks and other local services",
    rename_all = "kebab-case",
    author,
    version
)]
pub enum Command {
    #[clap(name = "run")]
    Run {},
}

impl Command {
    pub async fn execute(self, globals: Globals) -> Result<(), anyhow::Error> {
        match self {
            Command::Run {} => {
                // Verify that this process is the one running under file lock ~/tmp/.suibase-daemon.lock
                // If not, exit with an exit code of 13 (which is detected as "do no restart" by Suibase run-daemon.sh)
                // This is a safeguard against a user trying to start suibase-daemon without the proper locking.

                // Get the pid of this process.
                let my_pid = std::process::id();

                // Call the bash script "~/suibase/scripts/verify-suibase-daemon-lock $my_pid"

                let home_suibase = common::shared_types::get_home_suibase_path();
                let script_path = home_suibase.join("scripts/common/verify-suibase-daemon-lock.sh");

                // Call the bash script "~/suibase/scripts/verify-suibase-daemon-lock $my_pid"
                // Returns OK when the process is the one running under file lock ~/tmp/.suibase-daemon.lock
                let output = std::process::Command::new("/bin/bash")
                    .arg("-c")
                    .arg(format!("{} {}", script_path.display(), my_pid))
                    .output();

                let mut force_exit = true;

                if let Ok(output) = output {
                    if output.status.success() && output.stdout.starts_with(b"OK") {
                        force_exit = false;
                    }
                }

                if force_exit {
                    std::process::exit(13);
                }

                // Create mpsc channels (internal messaging between threads).
                //
                // The AdminController handles events about configuration changes
                //
                // The NetworkMonitor handles events about network stats and periodic health checks.
                //
                let (admctrl_tx, admctrl_rx) = tokio::sync::mpsc::channel(MPSC_Q_SIZE);
                let (netmon_tx, netmon_rx) = tokio::sync::mpsc::channel(MPSC_Q_SIZE);
                let (acoinsmon_tx, acoinsmon_rx) = tokio::sync::mpsc::channel(MPSC_Q_SIZE);

                // Instantiate and connect all subsystems (while none is "running" yet).
                let admctrl = AdminController::new(
                    globals.clone(),
                    admctrl_rx,
                    admctrl_tx.clone(),
                    netmon_tx.clone(),
                    acoinsmon_tx.clone(),
                );

                let netmon =
                    NetworkMonitor::new(globals.proxy.clone(), netmon_rx, netmon_tx.clone());

                let acoinsmon = ACoinsMonitor::new(
                    globals.config_devnet.clone(),
                    globals.config_testnet.clone(),
                    globals.status_devnet.clone(),
                    globals.status_testnet.clone(),
                    globals.config_mainnet.clone(),
                    globals.status_mainnet.clone(),
                    acoinsmon_rx,
                );

                let apiserver_params = APIServerParams::new(globals.clone(), admctrl_tx.clone());
                let apiserver = APIServer::new(apiserver_params);

                let clock_params = ClockTriggerParams::new(
                    netmon_tx.clone(),
                    acoinsmon_tx.clone(),
                    admctrl_tx.clone(),
                );
                let clock: ClockTrigger = ClockTrigger::new(clock_params);

                let suiexplorer_params =
                    WebserverParams::new(globals.clone(), admctrl_tx.clone(), "sui-explorer");
                let suiexplorer = WebserverWorker::new(suiexplorer_params);

                // Start all top levels subsystems.
                let errors = Toplevel::new(|s| async move {
                    s.start(SubsystemBuilder::new("admctrl", |a| admctrl.run(a)));
                    s.start(SubsystemBuilder::new("netmon", |a| netmon.run(a)));
                    s.start(SubsystemBuilder::new("acoinsmon", |a| acoinsmon.run(a)));
                    s.start(SubsystemBuilder::new("clock", |a| clock.run(a)));
                    s.start(SubsystemBuilder::new("suiexplorer", |a| suiexplorer.run(a)));
                    s.start(SubsystemBuilder::new("apiserver", |a| apiserver.run(a)));
                })
                .catch_signals()
                .handle_shutdown_requests(Duration::from_millis(1000))
                .await;

                if let Err(e) = &errors {
                    match e {
                        GracefulShutdownError::SubsystemsFailed(_) => {
                            log::error!("subsystems failed.")
                        }
                        GracefulShutdownError::ShutdownTimeout(_) => {
                            log::warn!("shutdown timed out.")
                        }
                    };

                    for subsystem_error in e.get_subsystem_errors() {
                        match subsystem_error {
                            SubsystemError::Failed(name, _e) => {
                                log::error!("   subsystem '{}' failed.", name);
                                /* TODO
                                match e.get_error() {
                                    SuibaseError::InternalError(data) => {
                                        log::error!("      Failed with SuibaseError::InternalError::WithData({})", data)
                                    }
                                }*/
                            }

                            SubsystemError::Panicked(name) => {
                                log::error!("   subsystem '{}' panicked.", name)
                            }
                        }
                    }
                }
                Ok(errors?)
            } // end Command::Run
        }
    }
} // end of Command

#[tokio::main]
async fn main() {
    // Initialize tracing with env filter support
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "poi_server=info,database=info,tower_http=warn".into()),
        )
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .compact() // Modern preference for daemon logs
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    
    // Allocate the globals "singleton".
    //
    // Globals are cloned by reference count.
    //
    // Keep a reference in main() so they will never get "deleted"
    // until the end of the program.
    let main_globals = Globals::new();

    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    Builder::from_env(Env::default().default_filter_or("info"))
        .filter(Some("jsonrpsee_server::server"), log::LevelFilter::Warn)
        .init();

    let cmd: Command = Command::parse();

    match cmd.execute(main_globals.clone()).await {
        Ok(_) => (),
        Err(err) => {
            log::error!("error: {}", err.to_string().red());
            std::process::exit(1);
        }
    }
}
