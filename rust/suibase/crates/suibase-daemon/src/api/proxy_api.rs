use axum::async_trait;

use jsonrpsee::core::RpcResult;

use crate::admin_controller::AdminControllerTx;
use crate::shared_types::Globals;

use super::ProxyApiServer;
use super::{LinkStats, LinksResponse};

pub struct ProxyApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
}

impl ProxyApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }
}

#[async_trait]
impl ProxyApiServer for ProxyApiImpl {
    async fn get_links(&self, _workdir: Option<String>) -> RpcResult<LinksResponse> {
        let mut resp = LinksResponse::new();

        // Get read-only access to the globals
        let globals_read_guard = self.globals.read().await;
        let globals = &*globals_read_guard;
        let input_ports = &globals.input_ports;

        // Find the matching input port, for now just pick the first one...
        if let Some(input_port) = input_ports.get(0) {
            resp.proxy_enabled = true;
            let target_servers = &input_port.target_servers;
            // Iterate the target servers and build a vector of LinkStats.
            let mut link_stats = Vec::new();
            for target_server in target_servers.iter() {
                let link_stat = LinkStats::new(target_server.1.uri().clone());
                link_stats.push(link_stat);
            }
            if !link_stats.is_empty() {
                resp.links = Some(link_stats);
            }
        }

        Ok(resp)
    }
}
