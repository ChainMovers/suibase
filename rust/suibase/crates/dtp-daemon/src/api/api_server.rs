// This is the server to handle the API of the dtp-daemon.
//
// Interaction with user is with JSON-RPC request/response messages.
//
// The APIServer is a thread that does a limited "sandboxing" of a
// single JSONRPCServer thread which auto-restart on panic.
//
// The server itself is from https://github.com/paritytech/jsonrpsee

use axum::async_trait;

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::{admin_controller::AdminControllerTx, shared_types::Globals};

use common::basic_types::{AutoThread, Runnable};

use super::GeneralApiServer;
use crate::api::impl_general_api::GeneralApiImpl;

use super::DtpApiServer;
use crate::api::impl_dtp_api::DtpApiImpl;

use super::PackagesApiServer;
use crate::api::impl_packages_api::PackagesApiImpl;

use jsonrpsee::{
    core::server::Methods,
    server::ServerBuilder,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct APIServerParams {
    globals: Globals,
    admctrl_tx: AdminControllerTx,
}

impl APIServerParams {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }
}

pub struct APIServer {
    auto_thread: AutoThread<APIServerThread, APIServerParams>,
}

impl APIServer {
    pub fn new(params: APIServerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("APIServer".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct APIServerThread {
    name: String,
    params: APIServerParams,
}

#[async_trait]
impl Runnable<APIServerParams> for APIServerThread {
    fn new(name: String, params: APIServerParams) -> Self {
        Self { name, params }
    }

    async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(_) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl APIServerThread {
    async fn event_loop(self, _subsys: &SubsystemHandle) -> Result<()> {
        // Reference:
        // https://github.com/paritytech/jsonrpsee/blob/master/examples/examples/cors_server.rs
        let cors = CorsLayer::new()
            // Allow `POST` when accessing the resource
            .allow_methods(axum::http::Method::POST)
            // Allow requests from any origin
            .allow_origin(Any)
            .allow_headers(vec![axum::http::header::CONTENT_TYPE]);
        let middleware = tower::ServiceBuilder::new().layer(cors);

        let builder = ServerBuilder::default()            
            .set_http_middleware(middleware);

        // TODO Put here the suibase.yaml proxy_port_number.
        let server = builder
            .build(SocketAddr::from(([127, 0, 0, 1], 44398)))
            .await?;

        let mut all_methods = Methods::new();

        {
            let api = DtpApiImpl::new(self.params.globals.clone(), self.params.admctrl_tx.clone());
            let methods = api.into_rpc();
            if let Err(e) = all_methods.merge(methods) {
                log::error!("Error merging ProxyApiImpl methods: {}", e);
            }
        }

        {
            let api =
                GeneralApiImpl::new(self.params.globals.clone(), self.params.admctrl_tx.clone());
            let methods = api.into_rpc();
            if let Err(e) = all_methods.merge(methods) {
                log::error!("Error merging GeneralApiImpl methods: {}", e);
            }
        }

        {
            let api =
                PackagesApiImpl::new(self.params.globals.clone(), self.params.admctrl_tx.clone());
            let methods = api.into_rpc();
            if let Err(e) = all_methods.merge(methods) {
                log::error!("Error merging ModulesApiImpl methods: {}", e);
            }
        }

        let handle = server.start(all_methods);
        handle.stopped().await;

        Ok(())
    }
}
