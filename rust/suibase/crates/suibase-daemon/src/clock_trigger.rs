use crate::globals::Globals;

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::network_monitor::{NetMonTx, NetworkMonitor};
use tokio::time::{interval, Duration};
pub struct ClockTrigger {
    _globals: Globals,
    netmon_tx: NetMonTx,
}

impl ClockTrigger {
    pub fn new(globals: Globals, netmon_tx: NetMonTx) -> Self {
        Self {
            _globals: globals,
            netmon_tx,
        }
    }

    async fn clock_loop(&mut self, subsys: &SubsystemHandle) {
        let mut interval = interval(Duration::from_secs(1));
        let mut tick: u64 = 0;
        loop {
            tick += 1;

            interval.tick().await;
            if subsys.is_shutdown_requested() {
                return;
            }

            if (tick % 10) == 0 {
                // Every 10 seconds
                let result = NetworkMonitor::send_event_globals_audit(&self.netmon_tx).await;
                if let Err(e) = result {
                    log::error!("send_event_globals_audit {}", e);
                    // TODO This is bad if sustain for many seconds. Add watchdog here.
                }
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        match self.clock_loop(&subsys).cancel_on_shutdown(&subsys).await {
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
