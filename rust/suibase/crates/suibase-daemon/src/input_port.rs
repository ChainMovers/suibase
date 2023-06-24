use crate::basic_types::*;
use crate::target_server::TargetServer;

pub struct InputPort {
    // Unique ID for this InputPort instance. Set once at construction (can never change).
    obj_id: InstanceID,

    // Fast access to data structure for this input port.
    //port_idx: ManagedVecUSize,

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
    pub target_servers: ManagedVec<TargetServer>,

    // Statistics (updated by the NetwworkMonitor).
    pub num_ok_req: u64,
    pub last_ok_req: EpochTimestamp, // Ignore when num_ok_req == 0

    pub num_failed_req: u64,
    pub last_failed_req: EpochTimestamp, // Ignore when num_failed_req == 0

    // Ignore when last_down_transition == last_up_transition
    pub last_down_transition: EpochTimestamp,
    pub last_up_transition: EpochTimestamp,
}

impl InputPort {
    pub fn new(port_number: u16) -> Self {
        let now = EpochTimestamp::now();

        Self {
            obj_id: gen_id(),
            //port_idx: INPUT_PORT_MAX,
            port_number,
            deactivate_request: false,
            proxy_server_running: false,
            healthy: false,
            target_servers: ManagedVec::new(),
            num_ok_req: 0,
            last_ok_req: now,
            num_failed_req: 0,
            last_failed_req: now,
            last_down_transition: now,
            last_up_transition: now,
        }
    }

    pub fn id(&self) -> InstanceID {
        self.obj_id
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

    pub fn find_best_target_server(&self) -> Option<(TargetServerIdx, String)> {
        let mut best_score = i8::MIN;
        let mut best_uri: String = String::new();
        let mut best_idx = None;

        for (i, target_server) in self.target_servers.iter() {
            let score = target_server.relative_health_score();
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
                best_uri = target_server.uri();
            }
        }
        if best_idx.is_none() {
            return None;
        }
        Some((best_idx.unwrap(), best_uri))
    }

    pub fn uri(&self, server_idx: TargetServerIdx) -> Option<String> {
        self.target_servers.get(server_idx).map(|ts| ts.uri())
    }
}
