use std::collections::HashMap;

use crate::basic_types::*;
use crate::target_server::TargetServer;

pub struct PortStates {
    // Unique ID for this PortStates instance. Set once at construction (can never change).
    id: PortMapID,

    // TCP/UDP port number. Set once at construction (can never change).
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

    // Periodically updated by the NetworkMonitor.
    pub healthy: bool,

    // Configuration. Can be change at runtime by the AdminController.
    pub target_servers: HashMap<IPKey, TargetServer>,

    // Statistics (updated by the NetwworkMonitor).
    pub num_ok_req: u64,
    pub last_ok_req: EpochTimestamp, // Ignore when num_ok_req == 0

    pub num_failed_req: u64,
    pub last_failed_req: EpochTimestamp, // Ignore when num_failed_req == 0

    // Ignore when last_down_transition == last_up_transition
    pub last_down_transition: EpochTimestamp,
    pub last_up_transition: EpochTimestamp,
}

impl PortStates {
    pub fn new(port_number: u16) -> Self {
        let now = EpochTimestamp::now();

        Self {
            id: gen_id(),
            port_number,
            deactivate_request: false,
            proxy_server_running: false,
            healthy: false,
            target_servers: HashMap::new(),
            num_ok_req: 0,
            last_ok_req: now,
            num_failed_req: 0,
            last_failed_req: now,
            last_down_transition: now,
            last_up_transition: now,
        }
    }

    pub fn id(&self) -> PortMapID {
        self.id
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

    pub fn find_best_target_server_uri(&self) -> Option<String> {
        let mut best_score = i8::MIN;
        let mut best_uri = None;

        for (_, target_server) in &self.target_servers {
            let score = target_server.relative_health_score();
            if score > best_score {
                best_score = score;
                best_uri = Some(target_server.uri());
            }
        }

        best_uri
    }
}
