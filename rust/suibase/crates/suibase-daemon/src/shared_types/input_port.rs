use crate::basic_types::*;
use crate::shared_types::TargetServer;

use super::ServerStats;

#[derive(Debug)]
pub struct InputPort {
    managed_idx: Option<ManagedVecUSize>,

    // The name of the workdir (e.g. localnet). Set once at contruction.
    workdir_name: String,

    // The workdir idx (from AdminController context). Set once at construction.
    workdir_idx: WorkdirIdx,

    // TCP/UDP port number. Set once at construction.
    port_number: u16,

    // Request that processing on this port be abandon.
    //
    // This is a irreversible request.
    //
    // This port configuration cannot be "re-activated" (the AdminController
    // must create another PortStates instance to re-use the same TCP/UDP port).
    deactivate_request: bool,

    // Indicate if a proxy_server thread is started or not for this port.
    proxy_server_running: bool,

    // Configuration. Can be change at runtime by the AdminController.
    pub target_servers: ManagedVec<TargetServer>,

    // Periodically updated by the NetworkMonitor.
    pub all_servers_stats: ServerStats,

    // The "TargetServer" selection vectors is updated periodically by
    // the NetworkMonitor. They help the handler to very quicly pick
    // a set of TargetServer to try.
    //
    // Design:
    //  All TargetServers in same selection[n] level are considered of same
    //  health quality even if their health score is slightly different.
    //
    //  The choice of one or another on same selection[n] level is related
    //  to load distribution.
    //
    //  The size of selection relates to the number of priority level defined
    //  by the user, and the real-time distribution of health_score.
    pub selection: Vec<Vec<TargetServerIdx>>,
}

impl InputPort {
    pub fn new(workdir_idx: WorkdirIdx, workdir_name: String, proxy_port_number: u16) -> Self {
        Self {
            managed_idx: None,
            workdir_name,
            workdir_idx,
            port_number: proxy_port_number,
            deactivate_request: false,
            proxy_server_running: false,
            target_servers: ManagedVec::new(),
            all_servers_stats: ServerStats::new("all".to_string()),
            selection: Vec::new(),
        }
    }

    pub fn add_target_server(&mut self, rpc: String, alias: String) {
        self.target_servers.push(TargetServer::new(rpc, alias));
    }

    pub fn workdir_idx(&self) -> WorkdirIdx {
        self.workdir_idx
    }

    pub fn workdir_name(&self) -> &str {
        &self.workdir_name
    }

    pub fn port_number(&self) -> u16 {
        self.port_number
    }

    pub fn deactivate(&mut self) {
        self.deactivate_request = true;
    }

    pub fn is_deactivated(&self) -> bool {
        self.deactivate_request
    }

    pub fn report_proxy_server_starting(&mut self) {
        self.proxy_server_running = true;
    }

    pub fn report_proxy_server_not_running(&mut self) {
        self.proxy_server_running = false;
    }

    fn find_best_target_server(&self) -> Option<(TargetServerIdx, String)> {
        let mut best_score: f64 = f64::MIN;
        let mut best_uri: String = String::new();
        let mut best_idx = None;

        for (i, target_server) in self.target_servers.iter() {
            let score = target_server.health_score();
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
                best_uri = target_server.uri();
            }
        }

        Some((best_idx?, best_uri))
    }

    pub fn get_best_target_servers(&self, target_servers: &mut Vec<(TargetServerIdx, String)>) {
        // Just leave target_servers untouch if there is any problem.
        // TODO Not implemented yet, just use the best one.
        if let Some(best) = self.find_best_target_server() {
            target_servers.push(best);
        }
    }

    pub fn uri(&self, server_idx: TargetServerIdx) -> Option<String> {
        self.target_servers.get(server_idx).map(|ts| ts.uri())
    }
}

impl ManagedElement for InputPort {
    fn managed_idx(&self) -> Option<ManagedVecUSize> {
        self.managed_idx
    }

    fn set_managed_idx(&mut self, index: Option<ManagedVecUSize>) {
        self.managed_idx = index;
    }
}
