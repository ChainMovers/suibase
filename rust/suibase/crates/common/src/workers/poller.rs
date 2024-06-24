// A PollerWorker does:
//   - Call into a PollingTrait to do a polling action (e.g. do a CLI command, update globals etc...)
//   - Handles AUDIT and UPDATE events. An audit is a *periodic* tentative polling, while
//     an UPDATE is an *on-demand* forced polling.
//
// The polling action is done from a tokio task and is auto-restarted on panic!
//
use std::sync::Arc;

use anyhow::Result;

use axum::async_trait;

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, NestedSubsystem, SubsystemBuilder, SubsystemHandle};

use crate::{
    basic_types::{
        self, remove_generic_event_dups, AutoThread, GenericChannelMsg, GenericRx, GenericTx,
        Instantiable, Runnable, WorkdirContext, WorkdirIdx, MPSC_Q_SIZE,
    },
    mpsc_q_check,
    shared_types::WORKDIRS_KEYS,
};

#[async_trait]
pub trait PollingTrait: Send {
    async fn update(&mut self);
}

#[allow(dead_code)]
// T: A "trait object" implementing PollingTrait
// P: The parameter needed to instantiate T.
pub struct PollerWorker<T, P> {
    params: P,
    poller_params: InnerPollerWorkerParams,
    poller_worker_handle: Option<NestedSubsystem<Box<dyn std::error::Error + Send + Sync>>>, // Set when the PollerWorker is started.
    polling_trait_obj: Arc<Mutex<T>>,
}

impl<T, P> WorkdirContext for PollerWorker<T, P>
where
    T: Instantiable<P> + PollingTrait + 'static,
    P: WorkdirContext + Clone,
{
    fn workdir_idx(&self) -> WorkdirIdx {
        self.params.workdir_idx()
    }
}

impl<T, P> PollerWorker<T, P>
where
    T: Instantiable<P> + PollingTrait + 'static,
    P: WorkdirContext + Clone,
{
    pub fn new(params: P, subsys: &SubsystemHandle) -> Self {
        let (poller_tx, poller_rx) = tokio::sync::mpsc::channel(MPSC_Q_SIZE);

        let polling_trait_obj = Arc::new(Mutex::new(T::new(params.clone())));

        let poller_params = InnerPollerWorkerParams::new(
            polling_trait_obj.clone() as Arc<Mutex<dyn PollingTrait>>,
            poller_rx,
            poller_tx.clone(),
            params.workdir_idx(),
        );

        let poller_worker = InnerPollerWorker::new(poller_params.clone());

        let handle = subsys.start(SubsystemBuilder::new(
            format!("poller-{}", params.workdir_idx()),
            |a| poller_worker.run(a),
        ));

        Self {
            params: params.clone(),
            poller_params,
            poller_worker_handle: Some(handle),
            polling_trait_obj,
        }
    }

    pub fn get_tx_channel(&self) -> GenericTx {
        self.poller_params.get_tx_channel()
    }

    pub fn get_polling_trait_obj(&self) -> Arc<Mutex<T>> {
        self.polling_trait_obj.clone()
    }
}

#[derive(Clone)]
pub struct InnerPollerWorkerParams {
    polling_object: Arc<Mutex<dyn PollingTrait>>,
    event_rx: Arc<Mutex<GenericRx>>, // To receive MSPC messages.
    event_tx: GenericTx,             // To send messages to self.
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl InnerPollerWorkerParams {
    pub fn new(
        polling_object: Arc<Mutex<dyn PollingTrait>>,
        event_rx: GenericRx,
        event_tx: GenericTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            polling_object,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            workdir_idx,
            workdir_name: WORKDIRS_KEYS[workdir_idx as usize].to_string(),
        }
    }

    pub fn get_tx_channel(&self) -> GenericTx {
        self.event_tx.clone()
    }
}

pub struct InnerPollerWorker {
    auto_thread: AutoThread<PollerWorkerTask, InnerPollerWorkerParams>,
}

impl InnerPollerWorker {
    pub fn new(params: InnerPollerWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new(
                format!("InnerPollerWorker-{}", params.workdir_name),
                params,
            ),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct PollerWorkerTask {
    task_name: String,
    params: InnerPollerWorkerParams,
    last_update_timestamp: Option<tokio::time::Instant>,
}

#[async_trait]
impl Runnable<InnerPollerWorkerParams> for PollerWorkerTask {
    fn new(task_name: String, params: InnerPollerWorkerParams) -> Self {
        Self {
            task_name,
            params,
            last_update_timestamp: None,
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("{} normal task exit (2)", self.task_name);
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("{} normal task exit (1)", self.task_name);
                Ok(())
            }
        }
    }
}

impl PollerWorkerTask {
    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        // This function is for periodic tentative update.
        if msg.event_id != crate::basic_types::EVENT_AUDIT {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Missing workdir_idx {:?}", msg);
            return;
        }

        let force = false;
        self.callback_update_in_trait_object(force).await;
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        // This function is for "on-demand" forced update.

        // Make sure the event_id is EVENT_UPDATE.
        if msg.event_id != crate::basic_types::EVENT_UPDATE {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
                return;
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        }

        let force = true;
        self.callback_update_in_trait_object(force).await;
    }

    async fn callback_update_in_trait_object(&mut self, force: bool) {
        if !force {
            // Debounce excessive update request because the callback will typically
            // be "expensive" and involve I/O.
            if let Some(last_cli_call_timestamp) = self.last_update_timestamp {
                if last_cli_call_timestamp.elapsed() < tokio::time::Duration::from_millis(50) {
                    return;
                }
            };
        }
        self.last_update_timestamp = Some(tokio::time::Instant::now());

        self.params.polling_object.lock().await.update().await;
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this task is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        // Remove duplicate of EVENT_AUDIT and EVENT_UPDATE in the MPSC queue.
        // (handle the case where the task was auto-restarted).
        remove_generic_event_dups(&mut event_rx, &self.params.event_tx);
        mpsc_q_check!(event_rx); // Just to help verify if the Q unexpectedly "accumulate".

        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = event_rx.recv().await {
                mpsc_q_check!(event_rx);
                match msg.event_id {
                    basic_types::EVENT_AUDIT => {
                        // Periodic processing.
                        self.process_audit_msg(msg).await;
                    }
                    basic_types::EVENT_UPDATE => {
                        // On-demand/reactive processing.
                        self.process_update_msg(msg).await;
                    }
                    _ => {
                        log::error!("Unexpected event_id {:?}", msg);
                    }
                }
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }
}
