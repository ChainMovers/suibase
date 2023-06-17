use crate::globals::Globals;
use anyhow::Result;

use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

pub struct NetmonMsg {
    // Internal messaging. Sent for every user request/response.
    // Purposely kept this <64 bytes for performance reason.
    para8: [u8; 2],   // Often [0]:event_id [1]:client_idx
    para16: [u16; 3], // Often [0]:port [1]:config_id [2]: time-to-response (ms)
    para32: [u32; 4], // Various stats.
}

impl NetmonMsg {
    pub fn new() -> Self {
        Self {
            para8: [0; 2],
            para16: [0; 3],
            para32: [0; 4],
        }
    }
}

pub type NetMonTx = crossbeam_channel::Sender<NetmonMsg>;
pub type NetMonRx = crossbeam_channel::Receiver<NetmonMsg>;

pub struct NetworkMonitor {
    // Configuration.
    pub enabled: bool,
    globals: Globals,
    netmon_rx: NetMonRx,
    netmon_tx: NetMonTx,
}

impl NetworkMonitor {
    pub fn new(globals: Globals, netmon_rx: NetMonRx, netmon_tx: NetMonTx) -> Self {
        Self {
            enabled: false,
            globals,
            netmon_rx,
            netmon_tx,
        }
    }

    async fn monitor_loop(&self, subsys: &SubsystemHandle) {
        loop {
            if subsys.is_shutdown_requested() {
                // Do a normal exit. This might not be needed, but here as abundance of caution.
                return;
            }
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        match self.monitor_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}
