// Generate periodical audit message toward other threads.
use anyhow::Result;
use axum::async_trait;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::{
    admin_controller::{AdminControllerMsg, AdminControllerTx},
    basic_types::{self, AutoThread, Runnable},
    network_monitor::{NetMonTx, NetworkMonitor},
};
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub struct ClockTriggerParams {
    netmon_tx: NetMonTx,
    admctrl_tx: AdminControllerTx,
}

impl ClockTriggerParams {
    pub fn new(netmon_tx: NetMonTx, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            netmon_tx,
            admctrl_tx,
        }
    }
}

pub struct ClockTrigger {
    auto_thread: AutoThread<ClockTriggerThread, ClockTriggerParams>,
}

impl ClockTrigger {
    pub fn new(params: ClockTriggerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("ClockTrigger".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct ClockTriggerThread {
    name: String,
    params: ClockTriggerParams,
}

#[async_trait]
impl Runnable<ClockTriggerParams> for ClockTriggerThread {
    fn new(name: String, params: ClockTriggerParams) -> Self {
        Self { name, params }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl ClockTriggerThread {
    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        let mut interval = interval(Duration::from_secs(1));
        let mut tick: u64 = 0;
        loop {
            tick += 1;

            interval.tick().await;
            if subsys.is_shutdown_requested() {
                return;
            }

            if (tick % 10) == 4 {
                // Every 10 seconds, with first one ~4 seconds after start.
                let result = NetworkMonitor::send_event_audit(&self.params.netmon_tx).await;
                if let Err(e) = result {
                    log::error!("send_event_globals_audit {}", e);
                    // TODO This is bad if sustain for many seconds. Add watchdog here.
                }
            }

            if (tick % 5) == 2 {
                // Every 5 seconds, with first one ~2 seconds after start.
                let mut msg = AdminControllerMsg::new();
                msg.event_id = basic_types::EVENT_AUDIT;
                let result = self.params.admctrl_tx.send(msg).await;
                if let Err(e) = result {
                    log::error!("admctrl_tx send_event_audit {}", e);
                    // TODO This is bad if sustain for many seconds. Add watchdog here.
                }
            }
        }
    }
}
