// Shared Global Variables
//
// Multi-threaded (tokio rt async) and Arc<RwLock> protected.
//
// Simple design:
//
//  - Each thread get a reference count (Arc) on the same multi-threaded 'MT' instance.
//
//  - A thread can lock read/write access on the single-thread 'ST' instance.
//
// Note: This app also uses message passing between threads to minimize sharing. See NetmonMsg as an example.
use std::sync::Arc;

use crate::api::{Versioned, VersionsResponse, WorkdirPackagesResponse, WorkdirStatusResponse};
use crate::shared_types::InputPort;
use common::basic_types::{ManagedVec, WorkdirIdx};
use common::shared_types::{
    GlobalsWorkdirConfigST, WORKDIR_IDX_DEVNET, WORKDIR_IDX_LOCALNET, WORKDIR_IDX_MAINNET,
    WORKDIR_IDX_TESTNET,
};

use super::GlobalsEventsDataST;

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

#[derive(Debug, Clone)]
pub struct GlobalsWorkdirPackagesST {
    // Mostly store everything in the same struct
    // as the response of the GetWorkdirPackages API. That way,
    // the UI queries can be served very quickly.
    pub ui: Option<Versioned<WorkdirPackagesResponse>>,
}

impl GlobalsWorkdirPackagesST {
    pub fn new() -> Self {
        Self { ui: None }
    }

    pub fn init_empty_ui(&mut self, workdir: String) {
        // As needed, initialize globals.ui with resp.
        let mut empty_resp = WorkdirPackagesResponse::new();
        empty_resp.header.method = "getWorkdirPackages".to_string();
        empty_resp.header.key = Some(workdir);

        let new_versioned_resp = Versioned::new(empty_resp.clone());
        // Copy the newly created UUID in the inner response header (so the caller can use these also).
        new_versioned_resp.write_uuids_into_header_param(&mut empty_resp.header);
        self.ui = Some(new_versioned_resp);
    }
}

impl Default for GlobalsWorkdirPackagesST {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GlobalsAPIMutexST {
    pub last_get_workdir_status_time: tokio::time::Instant,
    pub last_get_workdir_packages_time: tokio::time::Instant,
    pub last_versions_response: Option<Versioned<VersionsResponse>>,
}

impl GlobalsAPIMutexST {
    pub fn new() -> Self {
        Self {
            last_get_workdir_status_time: tokio::time::Instant::now(),
            last_get_workdir_packages_time: tokio::time::Instant::now(),
            last_versions_response: None,
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
}

impl GlobalsConfigST {
    pub fn new() -> Self {
        Self {
            daemon_ip: "localhost".to_string(),
            daemon_port: 44399,
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
pub type GlobalsConfigMT = Arc<tokio::sync::RwLock<GlobalsConfigST>>;
pub type GlobalsWorkdirConfigMT = Arc<tokio::sync::RwLock<GlobalsWorkdirConfigST>>;
pub type GlobalsWorkdirStatusMT = Arc<tokio::sync::RwLock<GlobalsWorkdirStatusST>>;
pub type GlobalsWorkdirPackagesMT = Arc<tokio::sync::RwLock<GlobalsWorkdirPackagesST>>;
pub type GlobalsEventsDataMT = Arc<tokio::sync::RwLock<GlobalsEventsDataST>>;
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

    // TODO: Refactor into an array with WorkdirIdx as index.

    // All workdirs configuration. Mostly reflects the suibase.yaml files and .state files.
    pub config_localnet: GlobalsWorkdirConfigMT,
    pub config_devnet: GlobalsWorkdirConfigMT,
    pub config_testnet: GlobalsWorkdirConfigMT,
    pub config_mainnet: GlobalsWorkdirConfigMT,

    // All workdirs status as presented on the UI (e.g. which process are running, is the localnet down?)
    pub status_localnet: GlobalsWorkdirStatusMT,
    pub status_devnet: GlobalsWorkdirStatusMT,
    pub status_testnet: GlobalsWorkdirStatusMT,
    pub status_mainnet: GlobalsWorkdirStatusMT,

    // Walrus stats per workdir
    pub walrus_stats_localnet: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
    pub walrus_stats_devnet: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
    pub walrus_stats_testnet: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,
    pub walrus_stats_mainnet: Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>,

    // Configuration related to Sui Move modules, particularly for package monitoring.
    pub packages_localnet: GlobalsWorkdirPackagesMT,
    pub packages_devnet: GlobalsWorkdirPackagesMT,
    pub packages_testnet: GlobalsWorkdirPackagesMT,
    pub packages_mainnet: GlobalsWorkdirPackagesMT,

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

    asui_selection: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(tokio::sync::RwLock::new(GlobalsProxyST::new())),
            config: Arc::new(tokio::sync::RwLock::new(GlobalsConfigST::new())),
            config_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new(
                WORKDIR_IDX_LOCALNET,
            ))),
            config_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new(
                WORKDIR_IDX_DEVNET,
            ))),
            config_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new(
                WORKDIR_IDX_TESTNET,
            ))),
            config_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new(
                WORKDIR_IDX_MAINNET,
            ))),
            status_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            status_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirStatusST::new())),
            walrus_stats_localnet: Arc::new(tokio::sync::RwLock::new(
                crate::walrus_monitor::WalrusStats::new(),
            )),
            walrus_stats_devnet: Arc::new(tokio::sync::RwLock::new(
                crate::walrus_monitor::WalrusStats::new(),
            )),
            walrus_stats_testnet: Arc::new(tokio::sync::RwLock::new(
                crate::walrus_monitor::WalrusStats::new(),
            )),
            walrus_stats_mainnet: Arc::new(tokio::sync::RwLock::new(
                crate::walrus_monitor::WalrusStats::new(),
            )),
            packages_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirPackagesST::new())),
            packages_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirPackagesST::new())),
            packages_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirPackagesST::new())),
            packages_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirPackagesST::new())),
            events_data_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            events_data_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsEventsDataST::new())),
            api_mutex_localnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_devnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_testnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            api_mutex_mainnet: Arc::new(tokio::sync::Mutex::new(GlobalsAPIMutexST::new())),
            asui_selection: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn get_config(&self, workdir_idx: WorkdirIdx) -> &GlobalsWorkdirConfigMT {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.config_localnet,
            WORKDIR_IDX_DEVNET => &self.config_devnet,
            WORKDIR_IDX_TESTNET => &self.config_testnet,
            WORKDIR_IDX_MAINNET => &self.config_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_status(&self, workdir_idx: WorkdirIdx) -> &GlobalsWorkdirStatusMT {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.status_localnet,
            WORKDIR_IDX_DEVNET => &self.status_devnet,
            WORKDIR_IDX_TESTNET => &self.status_testnet,
            WORKDIR_IDX_MAINNET => &self.status_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_packages(&self, workdir_idx: WorkdirIdx) -> &GlobalsWorkdirPackagesMT {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.packages_localnet,
            WORKDIR_IDX_DEVNET => &self.packages_devnet,
            WORKDIR_IDX_TESTNET => &self.packages_testnet,
            WORKDIR_IDX_MAINNET => &self.packages_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_api_mutex(&self, workdir_idx: WorkdirIdx) -> &GlobalsAPIMutexMT {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.api_mutex_localnet,
            WORKDIR_IDX_DEVNET => &self.api_mutex_devnet,
            WORKDIR_IDX_TESTNET => &self.api_mutex_testnet,
            WORKDIR_IDX_MAINNET => &self.api_mutex_mainnet,
            _ => {
                panic!("Invalid workdir_idx {}", workdir_idx)
            }
        }
    }

    pub fn get_walrus_stats(
        &self,
        workdir_name: &str,
    ) -> Option<&Arc<tokio::sync::RwLock<crate::walrus_monitor::WalrusStats>>> {
        match workdir_name {
            "localnet" => Some(&self.walrus_stats_localnet),
            "devnet" => Some(&self.walrus_stats_devnet),
            "testnet" => Some(&self.walrus_stats_testnet),
            "mainnet" => Some(&self.walrus_stats_mainnet),
            _ => None,
        }
    }

    pub fn events_data(&self, workdir_idx: WorkdirIdx) -> Option<&GlobalsEventsDataMT> {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&self.events_data_localnet),
            WORKDIR_IDX_DEVNET => Some(&self.events_data_devnet),
            WORKDIR_IDX_TESTNET => Some(&self.events_data_testnet),
            WORKDIR_IDX_MAINNET => Some(&self.events_data_mainnet),
            _ => None,
        }
    }
    pub fn events_data_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsEventsDataMT> {
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.events_data_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.events_data_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.events_data_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.events_data_mainnet),
            _ => None,
        }
    }

    pub async fn get_asui_selection(&self) -> Option<String> {
        let selection = self.asui_selection.lock().await;
        selection.clone()
    }

    pub async fn set_asui_selection(&mut self, new_value: Option<String>) {
        let mut selection = self.asui_selection.lock().await;
        *selection = new_value;
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}
