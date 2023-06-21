// Shared Global Variables
//
// Multi-threaded (tokio rt async) and Arc<RwLock> protected.
//
// Simple design:
//
//  - A single "all encompassing" RwLock for all global variables shared between the subsystems/threads
//    (AdminController, NetworkMonitor, ProxyServer etc...)
//
//  - Each thread get a reference count (Arc) on the same 'SafeGlobal' instance.
//
//  - A thread can choose read/write access to that 'SafeGlobal'
//
// Note: This app also uses message passing between threads to minimize sharing. See NetmonMsg as an example.
use std::sync::Arc;

use crate::basic_types::*;
use crate::input_port::InputPort;

pub struct PortMap {
    pub map: ManagedVec<InputPort>,
}

impl PortMap {
    pub fn new() -> Self {
        Self {
            map: ManagedVec::new(),
        }
    }
}

pub struct SafeGlobals {
    pub input_ports: PortMap,
}

impl SafeGlobals {
    pub fn new() -> Self {
        Self {
            input_ports: PortMap::new(),
        }
    }
}

pub type Globals = Arc<tokio::sync::RwLock<SafeGlobals>>;
