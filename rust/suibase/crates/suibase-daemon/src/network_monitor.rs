use crate::globals::Globals;
use anyhow::Result;

use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

pub struct NetmonMsg {
    // Internal messaging. Sent for every user request/response.
    // Purposely pack this in a few bytes for performance reason.
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

// Events related to a particular target servers.
// para8[1] will be the server_idx.
const EVENT_TRIG_TGT_AUDIT: i8 = 1; // Periodic check, including considering check for a RTT measurements.
const EVENT_REPORT_TGT_REQ_OK: i8 = 2; // Use by proxy_server to report stats to
const EVENT_REPORT_TGT_REQ_FAIL: i8 = 3;

impl std::fmt::Debug for NetmonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetmonMsg")
            .field("para8", &self.para8)
            .field("para16", &self.para16)
            .field("para32", &self.para32)
            .finish()
    }
}

pub type NetMonTx = tokio::sync::mpsc::Sender<NetmonMsg>;
pub type NetMonRx = tokio::sync::mpsc::Receiver<NetmonMsg>;

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

    async fn process_mut_globals(&mut self, msg: NetmonMsg) -> Option<NetmonMsg> {
        // Check if the message requires a global mutex.
        //let event = msg.para8[0];
        let mut globals_write_guard = self.globals.write().await;
        let globals = &mut *globals_write_guard;
        let input_port = &mut globals.input_ports;
        None
    }

    async fn monitor_loop(&mut self, subsys: &SubsystemHandle) {
        while let Some(i) = self.netmon_rx.recv().await {
            if subsys.is_shutdown_requested() {
                // Do a normal exit. Might not be needed, but done as abundance of caution.
                return;
            }

            // Do processing of message(s) that requires globals mutex (if any)
            let next_i = self.process_mut_globals(i).await;

            // Handle processing of one message that do not require globals mutex (if any).
            if next_i.is_some() {
                println!("got = {:?}", next_i.unwrap());
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
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
