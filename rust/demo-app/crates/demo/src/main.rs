// main.rs is just for handling the command line.
//
// See these other modules for the Sui related code.
mod counter;
mod events;

use clap::*;
use colored::Colorize;

#[allow(clippy::large_enum_variant)]
#[derive(Parser)]
#[clap(
    name = "demo",
    about = "A Rust SDK demo application with events subscription",
    rename_all = "kebab-case",
    author,
    version
)]
pub enum Command {
    #[clap(name = "events")]
    Events {},
    #[clap(name = "count")]
    Count {},
}

impl Command {
    pub async fn execute(self) -> Result<(), anyhow::Error> {
        match self {
            Command::Count {} => counter::count().await,
            Command::Events {} => {
                // Blocking until Ctrl-C or error.
                events::display_events_loop().await
            }
        }
    }
}

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();

    let cmd: Command = Command::parse();

    match cmd.execute().await {
        Ok(_) => (),
        Err(err) => {
            println!("{}", err.to_string().red());
            std::process::exit(1);
        }
    }
}
