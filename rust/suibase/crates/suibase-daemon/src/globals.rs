// Shared Global Variables
//
// Multi-threaded Arc tokio async RwLock protected.
//
// Simple design:
//
//  - A single "all encompassing" Mutex for all global variables shared between the subsystems/threads
//    (JsonRPCServer, ServerMonitor, ProxyServer etc...)
//
//  - Each thread get a reference count (Arc) on the same 'SafeGlobal' instance.
//
//  - A thread can choose read/write access to that 'SafeGlobal'
//
use std::collections::HashMap;
use std::sync::Arc;

pub type PortKey = u16;
pub type IPKey = String;

pub struct TargetServer {
    pub port: u16,
    pub host: String,
    pub healthy: bool,
    pub last_healthy: u64,
}

pub struct PortStates {
    pub healthy: bool,
    pub last_healthy: u64,
    pub target_servers: HashMap<IPKey, TargetServer>,
}

impl PortStates {
    pub fn new() -> Self {
        Self {
            healthy: false,
            last_healthy: 0,
            target_servers: HashMap::new(),
        }
    }
}
pub struct PortMapStates {
    pub map: HashMap<PortKey, PortStates>,
}

impl PortMapStates {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

pub struct SafeGlobals {
    pub port_states: PortMapStates,
}

impl SafeGlobals {
    pub fn new() -> Self {
        Self {
            port_states: PortMapStates::new(),
        }
    }
}

pub type Globals = Arc<tokio::sync::RwLock<SafeGlobals>>;
