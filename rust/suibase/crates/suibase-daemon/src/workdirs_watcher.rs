use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::admin_controller::{AdminControllerMsg, AdminControllerTx};
use crate::workdirs::Workdirs;

use notify::Watcher;
use notify::{Error, Event, RecommendedWatcher, RecursiveMode};

pub struct WorkdirsWatcher {
    workdirs: Workdirs,
    admctrl_tx: AdminControllerTx,
}

impl WorkdirsWatcher {
    pub fn new(workdirs: Workdirs, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            workdirs,
            admctrl_tx,
        }
    }

    async fn watch_loop(
        &mut self,
        subsys: &SubsystemHandle,
        mut local_rx: tokio::sync::mpsc::Receiver<notify::event::Event>,
    ) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = local_rx.recv().await {
                // Process the event from notify-rs
                log::info!("watch_loop() msg {:?}", msg);
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // Use a local channel to process "raw" events from notify-rs and then watch_loop()
        // translate them into higher level messages toward the AdminController.
        let (local_tx, local_rx) = tokio::sync::mpsc::channel::<notify::event::Event>(100);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                Ok(event) => {
                    //log::info!("watcher step 1 event {:?}", event);
                    let event_to_spawned_fn = event; //.clone();
                    let local_tx_to_spawned_fn = local_tx.clone();
                    rt.spawn(async move {
                        //log::info!("watcher step 2 event_clone {:?}", event_clone);
                        if let Err(e) = local_tx_to_spawned_fn.send(event_to_spawned_fn).await {
                            log::error!("local_tx.send {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("watcher error: {:?}", e);
                }
            })?;

        // Watch directories: ~/suibase then add watches on sub-directories as they are discovered.
        let path = self.workdirs.path();
        if path.exists() {
            let _ = watcher.watch(self.workdirs.path(), RecursiveMode::NonRecursive);
        } else {
            log::error!("implement watching above ~/suibase/workdirs for bad installation!");
        }

        for (_workdir_idx, workdir) in self.workdirs.workdirs.iter() {
            let path = workdir.path();
            // Check if path exists.
            if path.exists() {
                let _ = watcher.watch(workdir.path(), RecursiveMode::NonRecursive);
            }

            let path = workdir.state_path();
            if path.exists() {
                let _ = watcher.watch(workdir.state_path(), RecursiveMode::NonRecursive);
            }
        }
        log::info!("watcher {:?}", watcher);

        match self
            .watch_loop(&subsys, local_rx)
            .cancel_on_shutdown(&subsys)
            .await
        {
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
