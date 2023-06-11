// NOTE: THIS IS WORK IN PROGRESS. NOT READY FOR USE YET.
//
// main.rs does:
//  - Validate command line.
//  - Telemetry setup
//  - Top level subsystem starting:
//     - JsonRPCServer (the API)
//     - HttpServer (this application purpose!)
//     - ServerMonitor (thread to monitor configured target servers)

//mod tcp_server;
mod app_error;
mod globals;
mod http_server;
use std::sync::Arc;

use http_server::HttpServer;

use anyhow::Result;

use clap::*;

use colored::Colorize;
use pretty_env_logger::env_logger::{Builder, Env};

use tokio::time::{sleep, Duration};
use tokio_graceful_shutdown::{SubsystemHandle, Toplevel};

use globals::{Globals, SafeGlobals};

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
                // Create the default subsystems.
                let http_server = HttpServer::new(globals.clone());
                let json_rpc_server = JsonRPCServer::new(globals.clone());

                // Initialize the subsystems parameters here.

                // Start the subsystems.
                Toplevel::new()
                    .start("JsonRPCServer", |a| json_rpc_server.run(a))
                    .start("HttpServer", |a| http_server.run(a))
                    .catch_signals()
                    .handle_shutdown_requests(Duration::from_millis(1000))
                    .await
                    .map_err(Into::into)
            }
        }
    }
}

struct JsonRPCServer {
    // Configuration.
    enabled: bool,
    globals: Globals,
}

impl JsonRPCServer {
    pub fn new(globals: Globals) -> Self {
        Self {
            enabled: false,
            globals,
        }
    }

    async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        // This is going to be the API thread later... for now serve and do nothing,
        // just test our shutdown mechanism.
        log::info!("JsonRPCServer started. Enabled: {}", self.enabled);
        loop {
            sleep(Duration::from_millis(1000)).await;
            if subsys.is_shutdown_requested() {
                // Do a normal shutdown.
                log::info!("Shutting down JsonRPCServer (2).");
                return Ok(());
            }
        }

        // Task ends with an error. This should cause the main program to shutdown.
        // Err(anyhow!("JsonRPCServer threw an error."))

        // Normal shutdown:
        //   subsys.on_shutdown_requested().await;
        //   log::info!("Shutting down Subsystem1 ...");
        //   log::info!("Subsystem1 stopped.");
        //   Ok(())
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
