use common::basic_types::{
    AdminControllerMsg, AdminControllerTx, AutoSizeVec, WorkdirIdx, MPSC_Q_SIZE,
};

use anyhow::Result;
use common::shared_types::get_workdir_idx_by_path;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use notify::RecursiveMode;
use notify::{PollWatcher, Watcher};

use crate::Globals;

pub struct WorkdirsWatcher {
    globals: Globals,
    admctrl_tx: AdminControllerTx,
    tracking: AutoSizeVec<WorkdirTracking>,
}

#[derive(Default)]
struct WorkdirTracking {
    // Data private to the WorkdirsWatcher. One tracking per WorkdirIdx.
    is_workdir_watched: bool,
    is_state_watched: bool,
}

impl WorkdirsWatcher {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
            tracking: AutoSizeVec::new(),
        }
    }

    async fn send_notif_config_file_change(&self, path: String) {
        log::info!("Sending notif {}", path);
        let mut msg = AdminControllerMsg::new();
        msg.event_id = common::basic_types::EVENT_NOTIF_CONFIG_FILE_CHANGE;
        msg.data_string = Some(path);
        let _ = self.admctrl_tx.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
        });
    }

    // Return true if something newly watched/unwatched.
    fn update_workdir_watch(
        tracking: &mut AutoSizeVec<WorkdirTracking>,
        poll_watcher: &mut PollWatcher,
        workdir_idx: WorkdirIdx,
        target_path: &str,
    ) -> bool {
        // React only to change for two target_path: the workdir itself and its ".state"
        let paths = common::shared_types::get_workdir_paths(workdir_idx);
        let workdir_path = paths.workdir_root_path();
        let state_path = paths.state_path();
        if target_path != workdir_path.to_string_lossy()
            && target_path != state_path.to_string_lossy()
        {
            return false;
        }

        let mut at_least_one_modif: bool = false;

        // Synchronize the watcher with the states on the filesystem.
        let tracking = tracking.get_mut(workdir_idx);

        // Check if the path really exists.
        if !workdir_path.exists() {
            // If the path does not exist, then remove the watch.
            if tracking.is_workdir_watched {
                log::info!("unwatching {}", workdir_path.display());
                let _ = poll_watcher.unwatch(workdir_path);
                tracking.is_workdir_watched = false;
                at_least_one_modif = true;
            }
        } else {
            // The path exists, so add the watch (if not already done).
            // TODO Enhance this with FD tracking.
            if !tracking.is_workdir_watched {
                log::info!("watching {}", workdir_path.display());
                let _ = poll_watcher.watch(workdir_path, RecursiveMode::NonRecursive);
                tracking.is_workdir_watched = true;
                at_least_one_modif = true;
            }
        }

        if !state_path.exists() {
            // If the path does not exist, then remove the watch.
            if tracking.is_state_watched {
                log::info!("unwatching {}", state_path.display());
                let _ = poll_watcher.unwatch(state_path);
                tracking.is_state_watched = false;
                at_least_one_modif = true;
            }
        } else {
            // The path exists, so add the watch (if not already done).
            // TODO Enhance this with FD tracking?
            if !tracking.is_state_watched {
                log::info!("watching {}", state_path.display());
                let _ = poll_watcher.watch(state_path, RecursiveMode::NonRecursive);
                tracking.is_state_watched = true;
                at_least_one_modif = true;
            }
        }

        at_least_one_modif
    }

    fn remove_workdir_watch(
        tracking: &mut AutoSizeVec<WorkdirTracking>,
        poll_watcher: &mut PollWatcher,
        workdir_idx: WorkdirIdx,
        target_path: &str,
    ) -> bool {
        let mut at_least_one_modif: bool = false;

        // Synchronize the watcher with the states on the filesystem.
        let tracking = tracking.get_mut(workdir_idx);

        // React only to change for two target_path: the workdir itself and its ".state"
        let paths = common::shared_types::get_workdir_paths(workdir_idx);
        let workdir_path = paths.workdir_root_path();
        let state_path = paths.state_path();

        if target_path != workdir_path.to_string_lossy()
            && target_path != state_path.to_string_lossy()
        {
            return false;
        }

        // Check if the path really exists.
        if !workdir_path.exists() {
            // If the path does not exist, then remove the watch.
            if tracking.is_workdir_watched {
                log::info!("unwatching {}", workdir_path.display());
                let _ = poll_watcher.unwatch(workdir_path);
                tracking.is_workdir_watched = false;
                at_least_one_modif = true;
            }
        }

        if !state_path.exists() {
            // If the path does not exist, then remove the watch.
            if tracking.is_state_watched {
                log::info!("unwatching {}", state_path.display());
                let _ = poll_watcher.unwatch(state_path);
                tracking.is_state_watched = false;
                at_least_one_modif = true;
            }
        }

        at_least_one_modif
    }

    async fn watch_loop(
        &mut self,
        subsys: &SubsystemHandle,
        mut poll_watcher: PollWatcher,
        mut local_rx: tokio::sync::mpsc::Receiver<notify::event::Event>,
    ) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = local_rx.recv().await {
                common::mpsc_q_check!(local_rx);
                if msg.need_rescan() {
                    // TODO Implement rescan of all workdirs (assume events were missed).
                    log::error!("watch_loop() need_rescan (not implemented!)");
                }

                // Process the event from notify-rs
                //log::info!("watch_loop() msg {:?}", msg);
                // Iterate the msg.paths and find the workdir string (using Workdirs::find_workdir) and filename portion for each.
                match msg.kind {
                    notify::event::EventKind::Modify(_) => {
                        for path in msg.paths {
                            // Ignore everything except for user_request and suibase.yaml files.
                            if !path.ends_with("user_request") && !path.ends_with("suibase.yaml") {
                                continue;
                            }
                            self.send_notif_config_file_change(path.to_string_lossy().to_string())
                                .await
                        }
                    }
                    // notify::event::EventKind::Any()

                    // Meta-events about notifier itself (can be ignored).
                    // notify::event::EventKind::Other =>

                    // File creation, but not "writing" (can be ignored)
                    notify::event::EventKind::Create(create_kind) => {
                        // If creating one of the "suibase" standard workdir, then
                        // start watching it.
                        if create_kind == notify::event::CreateKind::Folder {
                            log::info!("CreateKind {:?}", msg);
                            for path in msg.paths {
                                let path = &path.to_string_lossy();
                                let workdir_idx = get_workdir_idx_by_path(path);
                                if workdir_idx.is_none() {
                                    continue;
                                }
                                let workdir_idx = workdir_idx.unwrap();

                                if Self::update_workdir_watch(
                                    &mut self.tracking,
                                    &mut poll_watcher,
                                    workdir_idx,
                                    path,
                                ) {
                                    // TODO Need to track creation of a few key file from here
                                    //      to make sure they are notified... for now always
                                    //      notified once after a delay with assumption the file
                                    //      were created after 1 second...
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                    self.send_notif_config_file_change(path.to_string()).await
                                }
                            }
                        }
                    }

                    notify::event::EventKind::Remove(remove_kind) => {
                        if remove_kind == notify::event::RemoveKind::Folder {
                            for path in msg.paths {
                                let path = &path.to_string_lossy();
                                let workdir_idx = get_workdir_idx_by_path(path);
                                if workdir_idx.is_none() {
                                    continue;
                                }
                                let workdir_idx = workdir_idx.unwrap();
                                if Self::remove_workdir_watch(
                                    &mut self.tracking,
                                    &mut poll_watcher,
                                    workdir_idx,
                                    path,
                                ) {
                                    self.send_notif_config_file_change(path.to_string()).await
                                }
                            }
                        }
                    }

                    // Access is for non-mutating operations (can be ignored)
                    // notify::event::EventKind::Access(_)

                    // notify::event::EventKind::Any
                    _ => {}
                }
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
        let (local_tx, local_rx) = tokio::sync::mpsc::channel::<notify::event::Event>(MPSC_Q_SIZE);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let poll_watcher_config = notify::Config::default();

        let mut poll_watcher = PollWatcher::new(
            move |res: Result<notify::Event, _>| match res {
                Ok(event) => {
                    //log::info!("watcher step 1 event {:?}", event);
                    let local_tx_to_spawned_fn = local_tx.clone();
                    rt.spawn(async move {
                        //log::info!("watcher step 2 event_clone {:?}", event_clone);
                        if let Err(e) = local_tx_to_spawned_fn.send(event).await {
                            log::error!("local_tx.send {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::warn!("{:?}", e);
                }
            },
            poll_watcher_config.with_poll_interval(std::time::Duration::from_secs(15)),
        )?;

        // Iterate WORKDIRS_KEYS and add watches on the directories.
        // Also add watches on the ".state" files.
        let mut add_root_path_done = false;
        for workdir_idx in 0..common::shared_types::WORKDIRS_KEYS.len() {
            let workdir_idx = workdir_idx as WorkdirIdx;
            // Watch directories: ~/suibase/workdirs then add watches on sub-directories as they are discovered.
            // TODO if suibase is deleted... then need to find a solution to recover gracefully (exit?).
            if !add_root_path_done {
                // This adds watch on ~/suibase/workdirs
                add_root_path_done = true;
                let path = common::shared_types::get_workdirs_path();
                if path.exists() {
                    let _ = poll_watcher.watch(path, RecursiveMode::NonRecursive);
                } else {
                    log::error!(
                        "implement watching above ~/suibase/workdirs for bad installation!"
                    );
                }
            }

            // This adds watch on ~/suibase/workdirs/<workdir> and ~/suibase/workdirs/<workdir>/.state
            let workdir_path = common::shared_types::get_workdir_paths(workdir_idx)
                .workdir_root_path()
                .to_string_lossy();
            if Self::update_workdir_watch(
                &mut self.tracking,
                &mut poll_watcher,
                workdir_idx as WorkdirIdx,
                &workdir_path,
            ) {
                self.send_notif_config_file_change(workdir_path.to_string())
                    .await;
            }
        }
        log::info!("watcher {:?}", poll_watcher);

        match self
            .watch_loop(&subsys, poll_watcher, local_rx)
            .cancel_on_shutdown(&subsys)
            .await
        {
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
