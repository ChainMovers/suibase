use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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

// An application target the localhost:port
//
// Each workdir should have a unique port assigned.
//
// The HashMap key is the port number.
//
#[derive(Clone)]
pub struct SharedStates {
    port_id: ManagedVecUSize,
    client: reqwest::Client,
    netmon_tx: NetMonTx,
    globals: Globals,
}

pub struct ProxyServer {
    enabled: bool,
    is_target_local: bool,
}

impl ProxyServer {
    pub fn new() -> Self {
        Self {
            enabled: false,
            is_target_local: false,
        }
    }

    async fn proxy_handler(
        State(states): State<Arc<tokio::sync::RwLock<SharedStates>>>,
        req: Request<Body>,
    ) -> Result<Response<Body>, AppError> {
        // Read access to the globals.
        let states_guard = states.read().await;
        let shared_state = &*states_guard;
        let globals_read_guard = shared_state.globals.read().await;
        let globals = &*globals_read_guard;

        // Build the request toward the best target server.
        let uri = if let Some(input_port) = globals.input_ports.map.get(shared_state.port_id) {
            input_port.find_best_target_server_uri()
        } else {
            None
        };

        if uri.is_none() {
            return Err(anyhow!("No server reacheable").into());
        }

        let req_builder = shared_state
            .client
            .request(req.method().clone(), uri.unwrap())
            .headers(req.headers().clone())
            .body(req.into_body());

        // Execute the request.
        let resp = req_builder.send().await?;

        // Handle the http::response
        let builder = Response::builder().body(Body::from(resp.bytes().await?));
        Ok(builder.unwrap())
    }

    pub async fn run(
        self,
        subsys: SubsystemHandle,
        port_id: ManagedVecUSize,
        globals: Globals,
        netmon_tx: NetMonTx,
    ) -> Result<()> {
        let shared_states: Arc<tokio::sync::RwLock<SharedStates>> =
            Arc::new(tokio::sync::RwLock::new(SharedStates {
                port_id,
                client: reqwest::Client::builder()
                    .no_proxy()
                    .connection_verbose(true)
                    .build()?,
                globals,
                netmon_tx,
            }));

        // Validate access to the PortStates in the Globals with an async confirmation that
        // there is a ProxyServer running for it (which will get clear on any failure to
        // start or later on any reason for thread exit).
        let port_number = {
            // Yes... it is amazingly complicated just to get access... but this is happening rarely
            // and is the price to pay to make "flexible and safe" multi-threaded globals in Rust.
            let mut states_guard = shared_states.write().await;
            let shared_state = &mut *states_guard;
            let mut globals_write_guard = shared_state.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_port = &mut globals.input_ports;
            if let Some(port_state) = input_port.map.get_mut(port_id) {
                port_state.report_proxy_server_starting();
                port_state.port_number()
            } else {
                log::error!("port {} not found", port_id);
                return Err(anyhow!("port {} not found", port_id));
            }
        };

        let app = Router::new()
            .fallback(get(Self::proxy_handler).post(Self::proxy_handler))
            .with_state(shared_states.clone());

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
            let mut states_guard = shared_states.write().await;
            let shared_state = &mut *states_guard;
            let mut globals_write_guard = shared_state.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_port = &mut globals.input_ports;
            if let Some(port_state) = input_port.map.get_mut(port_id) {
                port_state.report_proxy_server_not_running();
            }
        }

        return_value
    }
}
