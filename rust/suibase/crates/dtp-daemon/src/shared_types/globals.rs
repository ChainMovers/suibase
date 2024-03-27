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
//  - Although globals are not encouraged, they are carefully used here for data that
//    is often read, but relatively rarely changed. They are protected by a read-write lock.
//
// Note: This app also uses message passing between threads to minimize sharing. See NetmonMsg as an example.
use std::sync::Arc;

use crate::api::{Versioned, VersionsResponse, WorkdirStatusResponse};
use crate::shared_types::InputPort;
use common::basic_types::{GenericTx, ManagedVec, WorkdirIdx};

use super::{
    GlobalsDTPConnsStateClientST, GlobalsDTPConnsStateRxST, GlobalsDTPConnsStateServerST,
    GlobalsDTPConnsStateTxST, GlobalsEventsDataST, GlobalsPackagesConfigST, WebSocketWorkerIOTx,
    WebSocketWorkerTx,
};

use common::shared_types::{
    GlobalsWorkdirConfigST, GlobalsWorkdirsST, Workdir, WORKDIR_IDX_DEVNET, WORKDIR_IDX_LOCALNET,
    WORKDIR_IDX_MAINNET, WORKDIR_IDX_TESTNET,
};

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
pub struct GlobalsChannelsST {
    // Channels to transmit toward threads.
    // One instance of GlobalsChannelsST exists for every workdir.
    pub to_websocket_worker: Option<WebSocketWorkerTx>,
    pub to_websocket_worker_io: Option<WebSocketWorkerIOTx>,
    pub to_websocket_worker_tx: Option<GenericTx>,
    pub to_websocket_worker_rx: Option<GenericTx>,
}

impl GlobalsChannelsST {
    pub fn new() -> Self {
        Self {
            to_websocket_worker: None,
            to_websocket_worker_io: None,
            to_websocket_worker_tx: None,
            to_websocket_worker_rx: None,
        }
    }
}

impl std::default::Default for GlobalsChannelsST {
    fn default() -> Self {
        Self::new()
    }
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

// MT: Multi-threaded reference count, ST: Single-threaded access with a lock.
//
// Design Guidelines:
//  - A thread should NEVER hold more than one 'MT' *write* lock at the time.
//    This minimize potential for deadlocks.
//  - Release read lock ASAP (e.g. copy what you need and release).
//  - Release write lock even faster...

pub type GlobalsProxyMT = Arc<tokio::sync::RwLock<GlobalsProxyST>>;
pub type GlobalsWorkdirStatusMT = Arc<tokio::sync::RwLock<GlobalsWorkdirStatusST>>;
pub type GlobalsConfigMT = Arc<tokio::sync::RwLock<GlobalsWorkdirConfigST>>;
pub type GlobalsChannelsMT = Arc<tokio::sync::RwLock<GlobalsChannelsST>>;
pub type GlobalsPackagesConfigMT = Arc<tokio::sync::RwLock<GlobalsPackagesConfigST>>;
pub type GlobalsEventsDataMT = Arc<tokio::sync::RwLock<GlobalsEventsDataST>>;
pub type GlobalsWorkdirsMT = Arc<tokio::sync::RwLock<GlobalsWorkdirsST>>;
pub type GlobalsAPIMutexMT = Arc<tokio::sync::Mutex<GlobalsAPIMutexST>>;
pub type GlobalsDTPConnsStateClientMT = Arc<tokio::sync::RwLock<GlobalsDTPConnsStateClientST>>;
pub type GlobalsDTPConnsStateServerMT = Arc<tokio::sync::RwLock<GlobalsDTPConnsStateServerST>>;
pub type GlobalsDTPConnsStateTxMT = Arc<tokio::sync::RwLock<GlobalsDTPConnsStateTxST>>;
pub type GlobalsDTPConnsStateRxMT = Arc<tokio::sync::RwLock<GlobalsDTPConnsStateRxST>>;

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

    // Configuration driven by the user config (suibase.yaml) and actions (e.g. localnet start/stop).
    pub config_localnet: GlobalsConfigMT,
    pub config_devnet: GlobalsConfigMT,
    pub config_testnet: GlobalsConfigMT,
    pub config_mainnet: GlobalsConfigMT,

    // Channels toward some "permanent" threads (once set, a channel never
    // changes for the lifetime of the process).
    pub channels_localnet: GlobalsChannelsMT,
    pub channels_devnet: GlobalsChannelsMT,
    pub channels_testnet: GlobalsChannelsMT,
    pub channels_mainnet: GlobalsChannelsMT,

    // All path locations, plus some user common config that applies to all workdirs (e.g. port of this daemon).
    // These config are *rarely* changed for the lifetime of the process.
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

    // State of a DTP connection (e.g. open/closed)
    pub dtp_conns_state_client_localnet: GlobalsDTPConnsStateClientMT,
    pub dtp_conns_state_client_devnet: GlobalsDTPConnsStateClientMT,
    pub dtp_conns_state_client_testnet: GlobalsDTPConnsStateClientMT,
    pub dtp_conns_state_client_mainnet: GlobalsDTPConnsStateClientMT,

    pub dtp_conns_state_server_localnet: GlobalsDTPConnsStateServerMT,
    pub dtp_conns_state_server_devnet: GlobalsDTPConnsStateServerMT,
    pub dtp_conns_state_server_testnet: GlobalsDTPConnsStateServerMT,
    pub dtp_conns_state_server_mainnet: GlobalsDTPConnsStateServerMT,

    pub dtp_conns_state_tx_localnet: GlobalsDTPConnsStateTxMT,
    pub dtp_conns_state_tx_devnet: GlobalsDTPConnsStateTxMT,
    pub dtp_conns_state_tx_testnet: GlobalsDTPConnsStateTxMT,
    pub dtp_conns_state_tx_mainnet: GlobalsDTPConnsStateTxMT,

    pub dtp_conns_state_rx_localnet: GlobalsDTPConnsStateRxMT,
    pub dtp_conns_state_rx_devnet: GlobalsDTPConnsStateRxMT,
    pub dtp_conns_state_rx_testnet: GlobalsDTPConnsStateRxMT,
    pub dtp_conns_state_rx_mainnet: GlobalsDTPConnsStateRxMT,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(tokio::sync::RwLock::new(GlobalsProxyST::new())),
            config_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new())),
            config_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new())),
            config_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new())),
            config_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsWorkdirConfigST::new())),
            channels_localnet: Arc::new(tokio::sync::RwLock::new(GlobalsChannelsST::new())),
            channels_devnet: Arc::new(tokio::sync::RwLock::new(GlobalsChannelsST::new())),
            channels_testnet: Arc::new(tokio::sync::RwLock::new(GlobalsChannelsST::new())),
            channels_mainnet: Arc::new(tokio::sync::RwLock::new(GlobalsChannelsST::new())),
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
            dtp_conns_state_client_localnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateClientST::new(),
            )),
            dtp_conns_state_client_devnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateClientST::new(),
            )),
            dtp_conns_state_client_testnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateClientST::new(),
            )),
            dtp_conns_state_client_mainnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateClientST::new(),
            )),
            dtp_conns_state_server_localnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateServerST::new(),
            )),
            dtp_conns_state_server_devnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateServerST::new(),
            )),
            dtp_conns_state_server_testnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateServerST::new(),
            )),
            dtp_conns_state_server_mainnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateServerST::new(),
            )),
            dtp_conns_state_tx_localnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateTxST::new(),
            )),
            dtp_conns_state_tx_devnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateTxST::new(),
            )),
            dtp_conns_state_tx_testnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateTxST::new(),
            )),
            dtp_conns_state_tx_mainnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateTxST::new(),
            )),
            dtp_conns_state_rx_localnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateRxST::new(),
            )),
            dtp_conns_state_rx_devnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateRxST::new(),
            )),
            dtp_conns_state_rx_testnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateRxST::new(),
            )),
            dtp_conns_state_rx_mainnet: Arc::new(tokio::sync::RwLock::new(
                GlobalsDTPConnsStateRxST::new(),
            )),
        }
    }

    pub fn get_config(&self, workdir_idx: WorkdirIdx) -> &GlobalsConfigMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.config_localnet,
            WORKDIR_IDX_DEVNET => &self.config_devnet,
            WORKDIR_IDX_TESTNET => &self.config_testnet,
            WORKDIR_IDX_MAINNET => &self.config_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_channels(&self, workdir_idx: WorkdirIdx) -> &GlobalsChannelsMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.channels_localnet,
            WORKDIR_IDX_DEVNET => &self.channels_devnet,
            WORKDIR_IDX_TESTNET => &self.channels_testnet,
            WORKDIR_IDX_MAINNET => &self.channels_mainnet,
            _ => panic!("Invalid workdir_idx {}", workdir_idx),
        }
    }

    pub fn get_status(&self, workdir_idx: WorkdirIdx) -> &GlobalsWorkdirStatusMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.status_localnet,
            WORKDIR_IDX_DEVNET => &self.status_devnet,
            WORKDIR_IDX_TESTNET => &self.status_testnet,
            WORKDIR_IDX_MAINNET => &self.status_mainnet,
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

    pub fn events_data(&self, workdir_idx: WorkdirIdx) -> Option<&GlobalsEventsDataMT> {
        // Use hard coded workdir_idx to dispatch the right data.
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
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.events_data_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.events_data_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.events_data_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.events_data_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_client(&self, workdir_idx: WorkdirIdx) -> &GlobalsDTPConnsStateClientMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.dtp_conns_state_client_localnet,
            WORKDIR_IDX_DEVNET => &self.dtp_conns_state_client_devnet,
            WORKDIR_IDX_TESTNET => &self.dtp_conns_state_client_testnet,
            WORKDIR_IDX_MAINNET => &self.dtp_conns_state_client_mainnet,
            _ => {
                panic!("Invalid workdir_idx {}", workdir_idx)
            }
        }
    }

    pub fn dtp_conns_state_client_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsDTPConnsStateClientMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.dtp_conns_state_client_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.dtp_conns_state_client_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.dtp_conns_state_client_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.dtp_conns_state_client_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_server(&self, workdir_idx: WorkdirIdx) -> &GlobalsDTPConnsStateServerMT {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => &self.dtp_conns_state_server_localnet,
            WORKDIR_IDX_DEVNET => &self.dtp_conns_state_server_devnet,
            WORKDIR_IDX_TESTNET => &self.dtp_conns_state_server_testnet,
            WORKDIR_IDX_MAINNET => &self.dtp_conns_state_server_mainnet,
            _ => {
                panic!("Invalid workdir_idx {}", workdir_idx)
            }
        }
    }

    pub fn dtp_conns_state_server_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsDTPConnsStateServerMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.dtp_conns_state_server_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.dtp_conns_state_server_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.dtp_conns_state_server_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.dtp_conns_state_server_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_tx(&self, workdir_idx: WorkdirIdx) -> Option<&GlobalsDTPConnsStateTxMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&self.dtp_conns_state_tx_localnet),
            WORKDIR_IDX_DEVNET => Some(&self.dtp_conns_state_tx_devnet),
            WORKDIR_IDX_TESTNET => Some(&self.dtp_conns_state_tx_testnet),
            WORKDIR_IDX_MAINNET => Some(&self.dtp_conns_state_tx_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_tx_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsDTPConnsStateTxMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.dtp_conns_state_tx_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.dtp_conns_state_tx_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.dtp_conns_state_tx_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.dtp_conns_state_tx_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_rx(&self, workdir_idx: WorkdirIdx) -> Option<&GlobalsDTPConnsStateRxMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&self.dtp_conns_state_rx_localnet),
            WORKDIR_IDX_DEVNET => Some(&self.dtp_conns_state_rx_devnet),
            WORKDIR_IDX_TESTNET => Some(&self.dtp_conns_state_rx_testnet),
            WORKDIR_IDX_MAINNET => Some(&self.dtp_conns_state_rx_mainnet),
            _ => None,
        }
    }

    pub fn dtp_conns_state_rx_as_mut(
        &mut self,
        workdir_idx: WorkdirIdx,
    ) -> Option<&mut GlobalsDTPConnsStateRxMT> {
        // Use hard coded workdir_idx to dispatch the right data.
        match workdir_idx {
            WORKDIR_IDX_LOCALNET => Some(&mut self.dtp_conns_state_rx_localnet),
            WORKDIR_IDX_DEVNET => Some(&mut self.dtp_conns_state_rx_devnet),
            WORKDIR_IDX_TESTNET => Some(&mut self.dtp_conns_state_rx_testnet),
            WORKDIR_IDX_MAINNET => Some(&mut self.dtp_conns_state_rx_mainnet),
            _ => None,
        }
    }

    // Utility that returns the workdir_idx from the globals
    // using an exact workdir_name.
    //
    // This is a multi-thread safe call (will get the proper
    // lock on the globals).
    //
    // This is a relatively costly call, use wisely.
    pub async fn get_workdir_idx_by_name(&self, workdir_name: &String) -> Option<WorkdirIdx> {
        let workdirs_guard = self.workdirs.read().await;
        let workdirs = &*workdirs_guard;
        let workdirs_vec = &workdirs.workdirs;
        for (workdir_idx, workdir) in workdirs_vec.iter() {
            if workdir.name() == workdir_name {
                return Some(workdir_idx);
            }
        }
        None
    }

    // Utility that return a clone of the global Workdir for a given workdir_idx.
    // Multi-thread safe.
    // This is a relatively costly call, use wisely.
    pub async fn get_workdir_by_idx(&self, workdir_idx: WorkdirIdx) -> Option<Workdir> {
        let workdirs_guard = self.workdirs.read().await;
        let workdirs = &*workdirs_guard;
        let workdirs_vec = &workdirs.workdirs;
        if let Some(workdir) = workdirs_vec.get(workdir_idx) {
            return Some(workdir.clone());
        }
        None
    }

    // Utility to overwrite the global workdir config all at once.
    // Multi-thread safe.
    // This is a relatively costly call, use wisely.
    pub async fn set_workdir_config_by_idx(
        &mut self,
        workdir_idx: WorkdirIdx,
        workdir: GlobalsWorkdirConfigST,
    ) {
        let config = self.get_config(workdir_idx);
        let mut config_guard = config.write().await;
        *config_guard = workdir;
    }

    pub async fn get_workdir_by_name(
        &self,
        workdir_name: &String,
    ) -> Option<(WorkdirIdx, Workdir)> {
        let workdirs_guard = self.workdirs.read().await;
        let workdirs = &*workdirs_guard;
        let workdirs_vec = &workdirs.workdirs;
        for (workdir_idx, workdir) in workdirs_vec.iter() {
            if workdir.name() == workdir_name {
                return Some((workdir_idx, workdir.clone()));
            }
        }
        None
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}
