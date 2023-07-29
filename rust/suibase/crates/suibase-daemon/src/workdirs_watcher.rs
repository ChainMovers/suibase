use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE,
};
use crate::shared_types::Workdirs;

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

    async fn send_notif_config_file_change(&self, path: String) {
        log::info!("Sending config file change notification for {}", path);
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_NOTIF_CONFIG_FILE_CHANGE;
        msg.data_string = Some(path);
        let _ = self.admctrl_tx.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
        });
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
                // Iterate the msg.paths and find the workdir string (using Workdirs::find_workdir) and filename portion for each.
                // For each we will send_notif_config_file_change(workdir_name, filename) to the AdminController.

                // Identify if the event is meaningful.

                // Identify the related workdir.

                // Send a message to the AdminController.
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // Prime the AdminController with the current state of the workdirs.
        {
            let workdirs_guard = self.workdirs.read().await;
            let workdirs = &*workdirs_guard;

            for (_workdir_idx, workdir) in workdirs.workdirs.iter() {
                log::info!("Checking if started for {}", workdir.name());
                if workdir.is_user_request_start() {
                    self.send_notif_config_file_change(
                        workdir.suibase_yaml_default().to_string_lossy().to_string(),
                    )
                    .await;
                }
            }
        }

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

        {
            let workdirs_guard = self.workdirs.read().await;
            let workdirs = &*workdirs_guard;

            // Watch directories: ~/suibase then add watches on sub-directories as they are discovered.
            let path = workdirs.path();
            if path.exists() {
                let _ = watcher.watch(workdirs.path(), RecursiveMode::NonRecursive);
            } else {
                log::error!("implement watching above ~/suibase/workdirs for bad installation!");
            }

            for (_workdir_idx, workdir) in workdirs.workdirs.iter() {
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
        } // Release workdirs read lock

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
