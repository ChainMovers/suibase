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
//  - Although globals are not encouraged, they are carefully used here in a balanced way
//    and as a stepping stone toward a more optimized design. Ask the dev for more details.
//

// Note: This app also uses message passing between threads to minimize sharing. See NetmonMsg as an example.
use std::sync::Arc;

use crate::basic_types::ManagedVec;
use crate::shared_types::InputPort;

pub struct SafeGlobals {
    pub input_ports: ManagedVec<InputPort>,
}

impl SafeGlobals {
    pub fn new() -> Self {
        Self {
            input_ports: ManagedVec::new(),
        }
    }

    pub fn find_input_port_by_name(&self, workdir_name: &str) -> Option<&InputPort> {
        // Linear search in input_ports (vector size expected to remain small <5 elements)
        for input_port in self.input_ports.iter() {
            if input_port.1.workdir_name() == workdir_name {
                return Some(input_port.1);
            }
        }
        None
    }
}

impl Default for SafeGlobals {
    fn default() -> Self {
        Self::new()
    }
}

pub type Globals = Arc<tokio::sync::RwLock<SafeGlobals>>;
