use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::app_error::AppError;
use std::collections::HashMap;
use std::sync::Arc;

use crate::globals::{Globals, PortKey};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{uri::Uri, Request, Response},
    routing::get,
    Router,
};

use tokio_graceful_shutdown::SubsystemHandle;

use hyper::client::HttpConnector;

type Client = hyper::client::Client<HttpConnector, Body>;

// Define a Midclient ("Middleware Client").
//
// This is the client connecting to the *real* RPC server through HTTPS or other optimized means.
//
// Multiple MidClient instances can co-exist targeting the same real server (e.g. multiple workdir
// connecting to mainnet, or multiple apps in parallel running for same workdir).
//
#[derive(Clone)]
pub struct MidClient {
    pub client: Client,
}

impl MidClient {
    pub fn new() -> Self {
        Self {
            client: hyper::client::Client::builder().build(HttpConnector::new()),
        }
    }
}

// An application target the localhost:port
//
// Each workdir should have a unique port assigned.
//
// The HashMap key is the port number.
//
type PortMapMidClients = HashMap<PortKey, MidClient>;

#[derive(Clone)]
pub struct SharedStates {
    mid_clients: PortMapMidClients, // HashMap of MidClient by TCP port.
    globals: Globals,
}

impl SharedStates {
    pub fn new(globals: Globals) -> Self {
        Self {
            mid_clients: HashMap::new(),
            globals,
        }
    }
}
pub struct HttpServer {
    // Configuration.
    pub enabled: bool,
    pub shared_states: Arc<tokio::sync::RwLock<SharedStates>>,
}

impl HttpServer {
    pub fn new(globals: Globals) -> Self {
        Self {
            enabled: false,
            shared_states: Arc::new(tokio::sync::RwLock::new(SharedStates::new(globals))),
        }
    }

    async fn proxy_handler(
        State(states): State<Arc<tokio::sync::RwLock<SharedStates>>>,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, AppError> {
        let path = req.uri().path();
        let path_query = req
            .uri()
            .path_and_query()
            .map(|v| v.as_str())
            .unwrap_or(path);

        log::info!("Request called for {}", path.to_string());

        // Select a MidClient to use.
        let uri = format!("http://127.0.0.1:9123{}", path_query);

        *req.uri_mut() = Uri::try_from(uri).unwrap();

        // Write access to the shared states (among all handlers of this server).
        let mut states_guard = states.write().await;
        let shared_state = &mut *states_guard;

        // Read access to the globals (Proof of concept, not used yet)
        let globals_read_guard = shared_state.globals.read().await;
        let globals = &*globals_read_guard;
        let _port_states = &globals.port_states;

        // Find the MidClient. Add it if missing.
        let mid_clients = &shared_state.mid_clients;
        let mut mid_client: Option<&MidClient> = mid_clients.get(&9123);

        if mid_client.is_none() {
            mid_client = {
                let mid_clients = &mut shared_state.mid_clients;
                mid_clients.insert(9123, MidClient::new());
                log::info!("Adding Port 9123 !!!");
                mid_clients.get(&9123)
            }
        }

        // Final validation (no more retry).
        if mid_client.is_none() {
            return Err(AppError::from_str("No server available (no client)"));
        }

        Ok(mid_client.unwrap().client.request(req).await.unwrap())
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        let app = Router::new()
            .fallback(get(Self::proxy_handler).post(Self::proxy_handler))
            .with_state(self.shared_states);

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9124);
        log::info!("HttpServer listening on {}", bind_address);

        axum::Server::bind(&bind_address)
            .serve(app.into_make_service())
            .with_graceful_shutdown(subsys.on_shutdown_requested())
            .await
            .map_err(|err| anyhow! {err})
    }
}
