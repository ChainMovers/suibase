// This is the server to handle the API of the suibase-daemon.
//
// Interaction with user is with JSON-RPC request/response messages.
//
// The APIServer is a thread that does a limited "sandboxing" of a
// single JSONRPCServer thread which can be "auto-restarted" on panic.
//
// A JSONRPCServer owns a jsonrpsee Server to handle the JSON-RPC requests.
// ( https://github.com/paritytech/jsonrpsee )

use axum::async_trait;

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};
use tower::ServiceBuilder;

use crate::shared_types::Globals;

use common::{
    basic_types::{AdminControllerTx, AutoThread, Runnable},
    log_safe,
};

use super::GeneralApiServer;
use crate::api::impl_general_api::GeneralApiImpl;

use super::ProxyApiServer;
use crate::api::impl_proxy_api::ProxyApiImpl;

use super::PackagesApiServer;
use crate::api::impl_packages_api::PackagesApiImpl;

use jsonrpsee::{core::server::Methods, server::ServerBuilder};
use std::net::SocketAddr;
use tower_http::cors::AllowOrigin;

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
        log_safe!(format!("{} started", self.name));

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(_) => {
                log_safe!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log_safe!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl APIServerThread {
    async fn event_loop(self, _subsys: &SubsystemHandle) -> Result<()> {
        // Reference:
        // https://github.com/paritytech/jsonrpsee/blob/master/examples/examples/cors_server.rs
        let cors = tower_http::cors::CorsLayer::new()
            // Allow `POST` when accessing the resource
            .allow_methods(hyper::Method::POST)
            // Allow requests from any origin
            .allow_origin(AllowOrigin::any())
            .allow_headers([hyper::header::CONTENT_TYPE]);

        let service = ServiceBuilder::new().layer(cors);

        let server = ServerBuilder::default()
            .set_http_middleware(service)
            .build(SocketAddr::from(([127, 0, 0, 1], 44399)))
            .await?;

        let mut all_methods = Methods::new();

        {
            let api = ProxyApiImpl::new(
                self.params.globals.proxy.clone(),
                self.params.admctrl_tx.clone(),
            );
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
