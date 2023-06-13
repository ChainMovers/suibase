use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use std::collections::HashMap;
use std::sync::Arc;

use crate::app_error::AppError;
use crate::basic_types::*;
use crate::globals::Globals;
use crate::network_monitor::NetMonTx;

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
type PortMapMidClients = HashMap<PortMapID, MidClient>;

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
pub struct ProxyServer {
    enabled: bool,
    shared_states: Arc<tokio::sync::RwLock<SharedStates>>,
    netmon_tx: NetMonTx,
}

impl ProxyServer {
    pub fn new(globals: Globals, netmon_tx: NetMonTx) -> Self {
        Self {
            enabled: false,
            shared_states: Arc::new(tokio::sync::RwLock::new(SharedStates::new(globals))),
            netmon_tx,
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

        log::info!("request called for {}", path.to_string());

        // Select a MidClient to use.
        let uri = format!("http://127.0.0.1:9123{}", path_query);

        *req.uri_mut() = Uri::try_from(uri).unwrap();

        // Write access to the shared states (among all handlers of this server).
        let mut states_guard = states.write().await;
        let shared_state = &mut *states_guard;

        // Read access to the globals (Proof of concept, not used yet)
        let globals_read_guard = shared_state.globals.read().await;
        let globals = &*globals_read_guard;
        let _port_states = &globals.input_ports;

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
            return Err(AppError::from_str("no server available (no client)"));
        }

        Ok(mid_client.unwrap().client.request(req).await.unwrap())
    }

    pub async fn run(self, subsys: SubsystemHandle, port_id: PortMapID) -> Result<()> {
        // Validate access to the PortStates in the Globals with an async confirmation that
        // there is a ProxyServer running for it (which will get clear on any failure to
        // start or later on any reason for thread exit).
        let port_number = {
            // Yes... it is amazingly complicated just to set a bool variable... but this is happening rarely
            // and is the price to pay to make "flexible and safe" multi-threaded globals in Rust.
            let mut states_guard = self.shared_states.write().await;
            let shared_state = &mut *states_guard;
            let mut globals_write_guard = shared_state.globals.write().await;
            let globals = &mut *globals_write_guard;
            let port_states = &mut globals.input_ports;
            if let Some(port_state) = port_states.map.get_mut(&port_id) {
                port_state.report_proxy_server_starting();
                port_state.port_number()
            } else {
                log::error!("port {} not found", port_id);
                return Err(anyhow!("port {} not found", port_id));
            }
        };

        let app = Router::new()
            .fallback(get(Self::proxy_handler).post(Self::proxy_handler))
            .with_state(self.shared_states.clone());

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port_number);
        log::info!("listening on {}", bind_address);

        let return_value = axum::Server::bind(&bind_address)
            .serve(app.into_make_service())
            .with_graceful_shutdown(subsys.on_shutdown_requested())
            .await
            .map_err(|err| anyhow! {err});

        log::info!("stopped for {}", bind_address);

        {
            // This will cover for all scenario (abnormal or not) that the proxy had to exit. Will
            // allow the AdminController to detect and react as needed.
            let mut states_guard = self.shared_states.write().await;
            let shared_state = &mut *states_guard;
            let mut globals_write_guard = shared_state.globals.write().await;
            let globals = &mut *globals_write_guard;
            let port_states = &mut globals.input_ports;
            if let Some(port_state) = port_states.map.get_mut(&port_id) {
                port_state.report_proxy_server_not_running();
            }
        }

        return_value
    }
}
