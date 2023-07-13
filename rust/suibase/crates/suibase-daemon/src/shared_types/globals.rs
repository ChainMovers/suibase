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
}

impl Default for SafeGlobals {
    fn default() -> Self {
        Self::new()
    }
}

pub type Globals = Arc<tokio::sync::RwLock<SafeGlobals>>;
