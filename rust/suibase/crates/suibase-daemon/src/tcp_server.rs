use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

pub struct TCPServer {
    // Configuration.
    pub enabled: bool,
}

impl TCPServer {
    async fn listen_loop(&self, subsys: &SubsystemHandle) {
        // Start listening for incoming connection.
        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9124);
        let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();

        loop {
            if subsys.is_shutdown_requested() {
                // Do a normal exit. This might not be needed, but here as abundance of caution.
                return;
            }

            let (_stream, _addr) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Accept failed: {}", e);
                    continue;
                }
            };
            log::info!("Accepted connection from: {}", _addr);
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("TCPServer Listener started. Enabled: {}", self.enabled);

        match self.listen_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("TCPServer shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("TCPServer shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}
