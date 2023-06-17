// NOTE: THIS IS WORK IN PROGRESS. NOT READY FOR USE YET.
//
// main.rs does:
//  - Validate command line.
//  - Telemetry setup
//  - Top level threads started here:
//     - NetworkMonitor: Maintains various stats/status from messages fed from multiple sources. Uses crossbeam-channel.
//     - AdminController: The thread validating and applying the config, and also handle the JSON-RPC API.
//
// Other threads exists but are not started here:
//  - ProxyServer: Async handling of user traffic. One instance per listening port. Uses Axum/Hyper.
//                 Started by the AdminController.
//

use std::sync::Arc;

use anyhow::Result;

use clap::*;

use colored::Colorize;
use pretty_env_logger::env_logger::{Builder, Env};

use tokio::time::Duration;
use tokio_graceful_shutdown::Toplevel;

//mod tcp_server;
mod admin_controller;
mod app_error;
mod basic_types;
mod globals;
mod network_monitor;
mod port_states;
mod proxy_server;
mod server_stats;
mod target_server;

//use crate::basic_types::*;
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
                // Create internal messaging (between threads) channels.
                //
                // The NetworkMonitor receives network activities statistics from multiple producers.
                //
                // The reason that we use messaging is to minimize slowdown the user traffic for stats updates
                // (e.g. waiting on a global write Mutex on every request is not ideal).
                let (netmon_tx, netmon_rx) = crossbeam_channel::bounded(10000);

                // Instantiate all subsystems (while none is "running" yet).
                let admctrl = AdminController::new(globals.clone(), netmon_tx.clone());

                let netmon =
                    NetworkMonitor::new(globals.clone(), netmon_rx.clone(), netmon_tx.clone());

                // Start the subsystems.
                Toplevel::new()
                    .start("admctrl", move |a| admctrl.run(a))
                    .start("netmon", move |a| netmon.run(a))
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
    // From this point, Globals will only get "cloned" by reference count.
    //
    // Keep a reference here at main level so will never get "deleted" until the
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
            println!("{}", err.to_string().red());
            std::process::exit(1);
        }
    }
}
