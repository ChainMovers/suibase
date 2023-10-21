// Shared Global Variables
//
// Multi-threaded (tokio rt async) and Arc<RwLock> protected.
//
// Simple design:
//
//  - Group of global variables shared between the subsystems/threads
//    (AdminController, NetworkMonitor, ProxyServer etc...)
//
//  - Each thread get a reference count (Arc) on the same 'multi-thread 'MT' instance.
//
//  - A thread can lock read/write access on the single writer thread 'ST' instance.
//
//  - Although globals are not encouraged, they are carefully used here in a balanced way
//    and as a stepping stone toward a more optimized design. Ask the dev for more details.
//
// Note: This app also uses message passing between threads to minimize sharing. See NetmonMsg as an example.
use std::sync::Arc;

use crate::api::{StatusResponse, Versioned};
use crate::basic_types::{AutoSizeVec, ManagedVec};
use crate::shared_types::InputPort;

use super::{GlobalsEventsDataST, GlobalsPackagesConfigST, GlobalsWorkdirsST};

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
pub struct GlobalsWorkdirStatusST {
    // Mostly store everything in the same struct
    // as the response of the GetStatus API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<StatusResponse>>,
    pub last_ui_update: tokio::time::Instant,
}

impl GlobalsWorkdirStatusST {
    pub fn new() -> Self {
        Self {
            ui: None,
            last_ui_update: tokio::time::Instant::now(),
        }
    }
}

impl std::default::Default for GlobalsWorkdirStatusST {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsStatusST {
    // One per workdir, WorkdirIdx maintained by workdirs.
    pub workdirs: AutoSizeVec<GlobalsWorkdirStatusST>,
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
pub type GlobalsPackagesConfigMT = Arc<tokio::sync::RwLock<GlobalsPackagesConfigST>>;
pub type GlobalsEventsDataMT = Arc<tokio::sync::RwLock<GlobalsEventsDataST>>;
pub type GlobalsWorkdirsMT = Arc<tokio::sync::RwLock<GlobalsWorkdirsST>>;

// A convenient way to refer to all globals at once.
// Note: clone() conveniently increment the reference count of every field (ARC).
#[derive(Debug, Clone)]
pub struct Globals {
    // proxy server health status and various stats
    pub proxy: GlobalsProxyMT,

    // Configuration that rarely changes driven by suibase.yaml files (e.g. port of this daemon).
    pub config: GlobalsConfigMT,

    // All workdirs path locations and user controlled state (e.g. is localnet started by the user?).
    pub workdirs: GlobalsWorkdirsMT,

    // All workdirs status as presented on the UI (e.g. which process are running, is the localnet down?)
    pub status: GlobalsStatusMT,

    // Configuration related to Sui Move modules, particularly for monitoring management.
    pub packages_config: GlobalsPackagesConfigMT,

    // In-memory access to events data of actively monitored modules.
    pub events_data: GlobalsEventsDataMT,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(tokio::sync::RwLock::new(GlobalsProxyST::new())),
            config: Arc::new(tokio::sync::RwLock::new(GlobalsConfigST::new())),
            workdirs: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirsST::new())),
            status: Arc::new(tokio::sync::RwLock::new(GlobalsStatusST::new())),
            packages_config: Arc::new(tokio::sync::RwLock::new(GlobalsPackagesConfigST::new())),
            events_data: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
        }
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}
