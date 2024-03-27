use common::basic_types::{ManagedElement16, ManagedVecMapVec, ManagedVecU16};
use dtp_sdk::{Host, DTP};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
// One per host_sla_idx (driven by config).
//
// Used for variables operating on the server side. See DTPConnStateDataClient for the client side.
pub struct DTPConnStateDataServer {
    pub idx: Option<ManagedVecU16>,
    pub is_open: bool,
    pub dtp: Option<Arc<Mutex<DTP>>>,
    pub host: Option<Host>,
}

impl DTPConnStateDataServer {
    pub fn new() -> Self {
        Self {
            idx: None,
            is_open: false,
            dtp: None,
            host: None,
        }
    }

    pub fn set_dtp(&mut self, dtp: &Arc<Mutex<DTP>>) {
        self.dtp = Some(Arc::clone(dtp));
    }

    pub fn set_host(&mut self, host: Host) {
        self.host = Some(host);
    }
}

impl std::default::Default for DTPConnStateDataServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedElement16 for DTPConnStateDataServer {
    fn idx(&self) -> Option<ManagedVecU16> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU16>) {
        self.idx = index;
    }
}

#[derive(Debug)]
pub struct GlobalsDTPConnsStateServerST {
    pub conns: ManagedVecMapVec<DTPConnStateDataServer>,
}

impl GlobalsDTPConnsStateServerST {
    pub fn new() -> Self {
        Self {
            conns: ManagedVecMapVec::new(),
        }
    }
}

impl Default for GlobalsDTPConnsStateServerST {
    fn default() -> Self {
        Self::new()
    }
}
