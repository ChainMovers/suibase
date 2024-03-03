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

use crate::api::{Versioned, VersionsResponse, WorkdirStatusResponse};
use crate::shared_types::InputPort;
use common::basic_types::{AutoSizeVec, ManagedVec, WorkdirIdx};

use super::{workdirs, GlobalsEventsDataST, GlobalsPackagesConfigST, GlobalsWorkdirsST};

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
    // as the response of the GetWorkdirStatus API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<WorkdirStatusResponse>>,
}

impl GlobalsWorkdirStatusST {
    pub fn new() -> Self {
        Self { ui: None }
    }
}

impl std::default::Default for GlobalsWorkdirStatusST {
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
pub struct APIResponses {
    // Every response type for JSON-RPC clients.
    //
    // Used for:
    //  - delta detection and manage uuids.
    //  - micro-caching on excessive calls.
    //  - debugging
    //
    // versions are Versioned<> here, while all others are Versioned<> by its
    // original source variable (somewhere else in the globals).
    //
    pub versions: Option<Versioned<VersionsResponse>>,
    pub workdir_status: Option<WorkdirStatusResponse>,
}

impl APIResponses {
    pub fn new() -> Self {
        Self {
            versions: None,
            workdir_status: None,
        }
    }
}

impl Default for APIResponses {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GlobalsAPIMutexST {
    pub last_api_call_timestamp: tokio::time::Instant,
    pub last_responses: APIResponses,
}

impl GlobalsAPIMutexST {
    pub fn new() -> Self {
        Self {
            last_api_call_timestamp: tokio::time::Instant::now(),
            last_responses: APIResponses::default(),
        }
    }
}

impl Default for GlobalsAPIMutexST {
    fn default() -> Self {
        Self::new()
    }
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
            daemon_port: 44398,
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
pub type GlobalsWorkdirStatusMT = Arc<tokio::sync::RwLock<GlobalsWorkdirStatusST>>;
pub type GlobalsConfigMT = Arc<tokio::sync::RwLock<GlobalsConfigST>>;
pub type GlobalsPackagesConfigMT = Arc<tokio::sync::RwLock<GlobalsPackagesConfigST>>;
pub type GlobalsEventsDataMT = Arc<tokio::sync::RwLock<GlobalsEventsDataST>>;
pub type GlobalsWorkdirsMT = Arc<tokio::sync::RwLock<GlobalsWorkdirsST>>;
pub type GlobalsAPIMutexMT = Arc<tokio::sync::Mutex<GlobalsAPIMutexST>>;

// A convenient way to refer to all globals at once.
//
// clone() increment the reference count of every MT field (ARC).
// The caller then lock the MT field to access its ST content.
//
// Variables suffix meanings:
//   MT: Multi-threaded. Will require a lock() to access its ST content.
//   ST: Single-threaded. If you can code with it, then you can assume it is properly single threaded lock.
//
// The globals are also the container of what is mostly visible "outside" the process since
// they are often readable as-is through the JSON-RPC API, so be careful when doing changes.
//
#[derive(Debug, Clone)]
pub struct Globals {
    // proxy server health status and various stats
    pub proxy: GlobalsProxyMT,

    // Configuration that rarely changes driven by suibase.yaml files (e.g. port of this daemon).
    pub config: GlobalsConfigMT,

    // All workdirs path locations and user controlled state (e.g. is localnet started by the user?).
    pub workdirs: GlobalsWorkdirsMT,

    // All workdirs status as presented on the UI (e.g. which process are running, is the localnet down?)
    pub status_localnet: GlobalsWorkdirStatusMT,
    pub status_devnet: GlobalsWorkdirStatusMT,
    pub status_testnet: GlobalsWorkdirStatusMT,
    pub status_mainnet: GlobalsWorkdirStatusMT,

    // Configuration related to Sui Move modules, particularly for monitoring management.
    pub packages_config: GlobalsPackagesConfigMT,

    // In-memory access to events data of actively monitored modules.
    pub events_data_localnet: GlobalsEventsDataMT,
    pub events_data_devnet: GlobalsEventsDataMT,
    pub events_data_testnet: GlobalsEventsDataMT,
    pub events_data_mainnet: GlobalsEventsDataMT,

    // To avoid race conditions, all JSON-RPC API calls are serialized for a given workdir.
    pub api_mutex_localnet: GlobalsAPIMutexMT,
    pub api_mutex_devnet: GlobalsAPIMutexMT,
    pub api_mutex_testnet: GlobalsAPIMutexMT,
    pub api_mutex_mainnet: GlobalsAPIMutexMT,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(tokio::sync::RwLock::new(GlobalsProxyST::new())),
            config: Arc::new(tokio::sync::RwLock::new(GlobalsConfigST::new())),
            workdirs: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirsST::new())),
            status_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            packages_config: Arc::new(tokio::sync::RwLock::new(GlobalsPackagesConfigST::new())),
            events_data_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            api_mutex_localnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_devnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_testnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_mainnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
        }
    }

    pub fn get_status(&self, workdir_idx: WorkdirIdx) -> &GlobalsWorkdirStatusMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            workdirs::WORKDIR_IDX_LOCALNET => &self.status_localnet,
            workdirs::WORKDIR_IDX_DEVNET => &self.status_devnet,
            workdirs::WORKDIR_IDX_TESTNET => &self.status_testnet,
            workdirs::WORKDIR_IDX_MAINNET => &self.status_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_api_mutex(&self, workdir_idx: WorkdirIdx) -> &GlobalsAPIMutexMT {
        match workdir_idx {
            workdirs::WORKDIR_IDX_LOCALNET => &self.api_mutex_localnet,
            workdirs::WORKDIR_IDX_DEVNET => &self.api_mutex_devnet,
            workdirs::WORKDIR_IDX_TESTNET => &self.api_mutex_testnet,
            workdirs::WORKDIR_IDX_MAINNET => &self.api_mutex_mainnet,
            _ => {
                panic!("Invalid workdir_idx {}", workdir_idx)
            }
        }
    }

    pub fn events_data(&self, workdir_idx: WorkdirIdx) -> Option<&GlobalsEventsDataMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            workdirs::WORKDIR_IDX_LOCALNET => Some(&self.events_data_localnet),
            workdirs::WORKDIR_IDX_DEVNET => Some(&self.events_data_devnet),
            workdirs::WORKDIR_IDX_TESTNET => Some(&self.events_data_testnet),
            workdirs::WORKDIR_IDX_MAINNET => Some(&self.events_data_mainnet),
            _ => None,
        }
    }
    pub fn events_data_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsEventsDataMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            workdirs::WORKDIR_IDX_LOCALNET => Some(&mut self.events_data_localnet),
            workdirs::WORKDIR_IDX_DEVNET => Some(&mut self.events_data_devnet),
            workdirs::WORKDIR_IDX_TESTNET => Some(&mut self.events_data_testnet),
            workdirs::WORKDIR_IDX_MAINNET => Some(&mut self.events_data_mainnet),
            _ => None,
        }
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}
