// NOTE: THIS IS WORK IN PROGRESS. NOT READY FOR USE YET.
//
// main.rs does:
//  - Validate command line.
//  - Telemetry setup
//  - Top level threads started here. These runs until the program terminates:
//     - NetworkMonitor: Maintains remote server stats. Info coming from multiple sources (on a mpsc channel).
//     - AdminController: The thread validating and applying the config, and also handles the JSON-RPC API.
//     - ClockThread: Trigger periodic events (network stats recalculation, watchdog etc...)
//
// Other threads (not started here):
//
//  - ProxyServer: One long-running thread instance per listening port. Async handling of all user traffic.
//                 Uses axum::Server and reqwest::Client. Started/stopped by the AdminController.
//
//  - RequestWorker: Perform on-demand JSON-RPC requests for health check+latency test.
//                   Uses reqwest::Client. Started/stopped by the NetworkMonitor.
//

use std::sync::Arc;

use anyhow::Result;

use clap::*;

use clock_thread::ClockThread;
use colored::Colorize;
use pretty_env_logger::env_logger::{Builder, Env};

mod admin_controller;
mod app_error;
mod basic_types;
mod clock_thread;
mod globals;
mod input_port;
mod network_monitor;
mod proxy_server;
mod request_worker;
mod server_stats;
mod target_server;
mod workdirs;

use tokio::time::Duration;

use tokio_graceful_shutdown::Toplevel;

use crate::admin_controller::AdminController;
use crate::globals::{Globals, SafeGlobals};
use crate::network_monitor::NetworkMonitor;

#[allow(clippy::large_enum_variant)]
#[derive(Parser)]
#[clap(
    name = "suibase-daemon",
    about = "RPC and websockets proxy for more reliable access to Sui networks and other local services",
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
                // Create mpsc channels (internal messaging between threads).
                //
                // The AdminController handles events about configuration changes and API interactions.
                //
                // The NetworkMonitor handles events about network stats and periodic healtch checks.
                //
                let (admctrl_tx, admctrl_rx) = tokio::sync::mpsc::channel(100);
                let (netmon_tx, netmon_rx) = tokio::sync::mpsc::channel(10000);

                // Instantiate and connect all subsystems (while none is "running" yet).
                let admctrl = AdminController::new(
                    globals.clone(),
                    admctrl_rx,
                    admctrl_tx.clone(),
                    netmon_tx.clone(),
                );

                let netmon = NetworkMonitor::new(globals.clone(), netmon_rx, netmon_tx.clone());

                let clock: ClockThread = ClockThread::new(globals.clone(), netmon_tx.clone());

                // Start the subsystems.
                Toplevel::new()
                    .start("admctrl", move |a| admctrl.run(a))
                    .start("netmon", move |a| netmon.run(a))
                    .start("clock", move |a| clock.run(a))
                    .catch_signals()
                    .handle_shutdown_requests(Duration::from_millis(1000))
                    .await
                    .map_err(Into::into)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Allocate the globals "singleton".
    //
    // Globals are cloned by reference count.
    //
    // Keep a reference here at main level so it will never get "deleted" until the
    // end of the program.
    let main_globals = Arc::new(tokio::sync::RwLock::new(SafeGlobals::new()));

    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    //pretty_env_logger::init();
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let cmd: Command = Command::parse();

    match cmd.execute(main_globals.clone()).await {
        Ok(_) => (),
        Err(err) => {
            log::error!("error: {}", err.to_string().red());
            std::process::exit(1);
        }
    }
}
