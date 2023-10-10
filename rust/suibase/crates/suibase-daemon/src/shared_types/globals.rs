// Shared Global Variables
//
// Multi-threaded (tokio rt async) and Arc<RwLock> protected.
//
// Simple design:
//
//  - A single "all encompassing" RwLock for most global variables shared between the subsystems/threads
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

use crate::api::{StatusResponse, Versioned};
use crate::basic_types::{AutoSizeVec, ManagedVec};
use crate::shared_types::InputPort;

use super::{GlobalsWorkdirsMT, WorkdirsST};

#[derive(Debug)]
pub struct GlobalsProxyST {
    pub input_ports: ManagedVec<InputPort>,
}

impl GlobalsProxyST {
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

impl Default for GlobalsProxyST {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsStatusOneWorkdirST {
    // Mostly store everything in the same struct
    // as the response of the GetStatus API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<StatusResponse>>,
    pub last_ui_update: tokio::time::Instant,
}

impl GlobalsStatusOneWorkdirST {
    pub fn new() -> Self {
        Self {
            ui: None,
            last_ui_update: tokio::time::Instant::now(),
        }
    }
}

impl std::default::Default for GlobalsStatusOneWorkdirST {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsStatusST {
    // One per workdir, WorkdirIdx maintained by workdirs.
    pub workdirs: AutoSizeVec<GlobalsStatusOneWorkdirST>,
}

impl GlobalsStatusST {
    pub fn new() -> Self {
        Self {
            workdirs: AutoSizeVec::new(),
        }
    }
}

impl Default for GlobalsStatusST {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct GlobalsWorkdirConfigST {
    // These are variables that rarely changes and
    // are controlled by the user (suibase.yaml
    // files, workdir CLI operations).
    pub precompiled_bin: bool,
}

#[derive(Debug)]
pub struct GlobalsConfigST {
    // These are variables that rarely changes and
    // are controlled by the user (suibase.yaml
    // files, workdir CLI operations).
    pub daemon_ip: String,
    pub daemon_port: u16,

    // Uses same index as the ManagedVec WorkdirsST::workdirs
    pub workdirs: AutoSizeVec<GlobalsWorkdirConfigST>,
}

impl GlobalsConfigST {
    pub fn new() -> Self {
        Self {
            daemon_ip: "0.0.0.0".to_string(),
            daemon_port: 44399,
            workdirs: AutoSizeVec::new(),
        }
    }
}

impl Default for GlobalsConfigST {
    fn default() -> Self {
        Self::new()
    }
}

// MT: Multi-threaded reference count, ST: Single-threaded access with a lock.
//
// Design Guidelines:
//  - A thread should NEVER hold more than one 'MT' *write* lock at the time.
//    This minimize potential for deadlocks.
//  - Release read lock ASAP (e.g. copy what you need and release).
//  - Release write lock even faster...

pub type GlobalsProxyMT = Arc<tokio::sync::RwLock<GlobalsProxyST>>;
pub type GlobalsStatusMT = Arc<tokio::sync::RwLock<GlobalsStatusST>>;
pub type GlobalsConfigMT = Arc<tokio::sync::RwLock<GlobalsConfigST>>;

// A convenient way to refer to all globals at once.
// Note: clone() conveniently increment the reference count of every field (ARC).
#[derive(Debug, Clone)]
pub struct Globals {
    pub proxy: GlobalsProxyMT,
    pub config: GlobalsConfigMT,
    pub workdirs: GlobalsWorkdirsMT,
    pub status: GlobalsStatusMT,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(tokio::sync::RwLock::new(GlobalsProxyST::new())),
            config: Arc::new(tokio::sync::RwLock::new(GlobalsConfigST::new())),
            workdirs: Arc::new(tokio::sync::RwLock::new(WorkdirsST::new())),
            status: Arc::new(tokio::sync::RwLock::new(GlobalsStatusST::new())),
        }
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}
