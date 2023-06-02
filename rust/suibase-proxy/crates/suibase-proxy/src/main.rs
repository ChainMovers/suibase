// NOTE: THIS IS WORK IN PROGRESS. NOT READY FOR USE YET.
//
// main.rs does:
//  - Validate command line.
//  - Telemetry setup
//  - Top level subsystem starting:
//     - JsonRPCServer (the API)
//     - TcpProxyServer (this application purpose!)
//     - ServicesMonitor (thread to monitor other remote services)

mod tcp_server;

use tcp_server::TCPServer;

use anyhow::Result;

use clap::*;

use colored::Colorize;
use env_logger::{Builder, Env};

use tokio::time::{sleep, Duration};
use tokio_graceful_shutdown::{SubsystemHandle, Toplevel};

#[allow(clippy::large_enum_variant)]
#[derive(Parser)]
#[clap(
    name = "suibase-proxy",
    about = "RPC and websockets proxy for more reliable access to Sui networks",
    rename_all = "kebab-case",
    author,
    version
)]
pub enum Command {
    #[clap(name = "run")]
    Run {},
}

impl Command {
    pub async fn execute(self) -> Result<(), anyhow::Error> {
        match self {
            Command::Run {} => {
                // Initialize the subsystems parameters here.
                let json_rpc_server = JsonRPCServer { enabled: true };
                let tcp_server = TCPServer { enabled: true };

                // Start the subsystems.
                Toplevel::new()
                    .start("JsonRPCServer", |a| json_rpc_server.run(a))
                    .start("TCPServer", |a| tcp_server.run(a))
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
}

impl JsonRPCServer {
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
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let cmd: Command = Command::parse();

    match cmd.execute().await {
        Ok(_) => (),
        Err(err) => {
            println!("{}", err.to_string().red());
            std::process::exit(1);
        }
    }
}
