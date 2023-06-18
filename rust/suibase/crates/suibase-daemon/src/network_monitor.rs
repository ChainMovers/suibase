use crate::basic_types::*;

use crate::globals::Globals;
use bitflags::bitflags;

use anyhow::{anyhow, Result};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

pub struct NetmonMsg {
    // Internal messaging. Sent for every user request/response.
    // Purposely pack this in a few bytes for performance reason.
    para8: [u8; 4], // Often [0]:event_id [1]: Flags  [2]:InputPortIdx  [3]:TargetServerIdx
    para32: [u32; 3], // Various stats.
}

impl NetmonMsg {
    pub fn new() -> Self {
        Self {
            para8: [0; 4],
            para32: [0; 3],
        }
    }
}

// Events ID
//
const EVENT_TRIG_TGT_AUDIT: u8 = 1; // Periodic check, including considering check for a RTT measurements.
const EVENT_REPORT_TGT_REQ_OK: u8 = 2; // proxy_server reporting stats on successul request/response.
const EVENT_REPORT_TGT_REQ_FAIL: u8 = 3;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct NetmonFlags: u8 {
        const NEED_GLOBAL_WRITE_MUTEX = 0x80;
        const NEED_GLOBAL_READ_MUTEX = 0x40;
    }
}

impl std::fmt::Debug for NetmonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetmonMsg")
            .field("para8", &self.para8)
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

    pub async fn report_proxy_handler_resp_ok(
        tx_channel: &NetMonTx,
        port_idx: InputPortIdx,
        server_idx: TargetServerIdx,
        handler_start: EpochTimestamp,
        send_start: EpochTimestamp,
        resp_end: EpochTimestamp,
    ) -> Result<()> {
        let mut msg = NetmonMsg::new();
        msg.para8[0] = EVENT_REPORT_TGT_REQ_OK;
        msg.para8[1] = NetmonFlags::NEED_GLOBAL_WRITE_MUTEX.bits();
        msg.para8[2] = port_idx;
        msg.para8[3] = server_idx;
        msg.para32[0] = {
            match (send_start - handler_start).as_millis().try_into() {
                Ok(value) => value,
                Err(_) => 1,
            }
        };
        msg.para32[1] = {
            match (resp_end - send_start).as_millis().try_into() {
                Ok(value) => value,
                Err(_) => 1,
            }
        };

        // Send the message.
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("send failed {}", e);
            anyhow!("send failed {}", e)
        })
    }

    async fn process_mut_globals(&mut self, mut msg: NetmonMsg) -> Option<NetmonMsg> {
        if msg.para8[1] & NetmonFlags::NEED_GLOBAL_WRITE_MUTEX.bits() == 0 {
            // Do not consume the message.
            return Some(msg);
        }

        {
            let mut globals_write_guard = self.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_port = &mut globals.input_ports;
            loop {
                // Consume the message.
                match msg.para8[0] {
                    EVENT_TRIG_TGT_AUDIT => {
                        log::info!("EVENT_TRIG_TGT_AUDIT");
                    }
                    EVENT_REPORT_TGT_REQ_OK => {
                        log::info!("EVENT_REPORT_TGT_REQ_OK");
                    }
                    EVENT_REPORT_TGT_REQ_FAIL => {
                        log::info!("EVENT_REPORT_TGT_REQ_FAIL");
                    }
                    _ => {
                        // Should not happen.
                        panic!("unexpected event id {}", msg.para8[0]);
                    }
                }
                // Check if more messages are available.
                match self.netmon_rx.try_recv() {
                    Ok(next_msg) => {
                        msg = next_msg;
                    }
                    Err(_e) => {
                        // No more messages.
                        return None;
                    }
                }

                if msg.para8[1] & NetmonFlags::NEED_GLOBAL_WRITE_MUTEX.bits() == 0 {
                    // Does not requires a global mutex.
                    // Do not consume that message here.
                    return Some(msg);
                }
            }
        }
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
