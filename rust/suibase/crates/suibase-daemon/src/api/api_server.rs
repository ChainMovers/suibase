// This is the server to handle the API of the suibase-daemon.
//
// Interaction with user is with JSON-RPC request/response messages.
//
// The APIServer is a thread that does a limited "sandboxing" of a
// single JSONRPCServer thread which can be "auto-restarted" on panic.
//
// A JSONRPCServer owns a jsonrpsee Server to handle the JSON-RPC requests.
// ( https://github.com/paritytech/jsonrpsee )

use reqwest::Proxy;
use tokio::time::{interval, Duration};

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle, Toplevel};

use crate::{admin_controller::AdminControllerTx, shared_types::Globals};

use super::ProxyApiServer;
use crate::api::proxy_api::ProxyApiImpl;

use hyper::Method;
use jsonrpsee::server::{AllowHosts, RpcModule, Server, ServerBuilder};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

pub struct APIServer {
    globals: Globals,
    admctrl_tx: AdminControllerTx,
}

impl APIServer {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("APIServer started");
        loop {
            let inner_server: JSONRPCServer =
                JSONRPCServer::new(self.globals.clone(), self.admctrl_tx.clone());

            // Create an instance of JSONRPCServer. If it panics, then
            // we will just create a new instance ("start a new one" == "restart").
            let inner_server_result = Toplevel::nested(&subsys, "")
                .start("inner", |a| inner_server.run(a))
                .handle_shutdown_requests(Duration::from_millis(50))
                .await;

            if let Err(err) = &inner_server_result {
                // TODO Restart the process on excess of errors for tentative recovery (e.g. memory leaks?)
                log::error!("JSONRPCServer server: {}", err);
                // Something went wrong, wait a couple of second before restarting
                // the inner server, but do not block from exiting.
                for _ in 0..4 {
                    if subsys.is_shutdown_requested() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }

            if subsys.is_shutdown_requested() {
                break;
            }

            log::info!("Restarting JSON-RPC server ...");
        }
        log::info!("APIServer shutting down - normal exit");
        Ok(())
    }
}

struct JSONRPCServer {
    globals: Globals,
    admctrl_tx: AdminControllerTx,
}

impl JSONRPCServer {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }

    async fn run_server(self, _subsys: &SubsystemHandle) -> Result<()> {
        // Reference:
        // https://github.com/paritytech/jsonrpsee/blob/master/examples/examples/cors_server.rs
        let cors = CorsLayer::new()
            // Allow `POST` when accessing the resource
            .allow_methods([Method::POST])
            // Allow requests from any origin
            .allow_origin(Any)
            .allow_headers([hyper::header::CONTENT_TYPE]);
        let middleware = tower::ServiceBuilder::new().layer(cors);

        let builder = ServerBuilder::default()
            .batch_requests_supported(false)
            .set_host_filtering(AllowHosts::Any)
            .set_middleware(middleware);

        // TODO Put here the suibase.yaml proxy_port_number.
        let server = builder
            .build(SocketAddr::from(([127, 0, 0, 1], 44399)))
            .await?;

        let proxy_api = ProxyApiImpl::new(self.globals.clone(), self.admctrl_tx.clone());
        let methods = proxy_api.into_rpc();

        let start_result = server.start(methods);

        if let Ok(handle) = start_result {
            //let addr = server.local_addr()?;
            //log::info!(local_addr =? addr, "JSON-RPC server listening on {addr}");
            //log::info!("Available JSON-RPC methods : {:?}", module.method_names());
            // Wait for the server to finish. This will block until
            // CancelledByShutdown.
            handle.stopped().await;
        } else {
            log::error!("JSONRPSEE failed to start");
        }

        Ok(())
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("JSONRPCServer server started");

        match self.run_server(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(_) => {
                log::info!("JSONRPCServer server shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("JSONRPCServer server shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}
