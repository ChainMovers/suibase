// Child task of admin_controller
//
// One instance per workdir.
//
// Responsible to:
//  - Periodically and on-demand check published packages
//    under ~/suibase/workdirs and update globals.
//
// The task is auto-restart in case of panic.

use anyhow::anyhow;
use common::{
    basic_types::{Instantiable, WorkdirContext, WorkdirIdx},
    log_safe,
    shared_types::WORKDIRS_KEYS,
    workers::{PollerWorker, PollingTrait},
};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::{
    api::{PackageInstance, SuiObjectInstance, SuiObjectType},
    shared_types::{Globals, PackagePath},
};

use anyhow::Result;

use axum::async_trait;
use common::basic_types::{self, GenericChannelMsg, GenericTx};

use tokio_graceful_shutdown::SubsystemHandle;

#[derive(Clone)]
pub struct PackagesPollerParams {
    globals: Globals,
    sui_events_worker_tx: Option<GenericTx>, // To send messages to related Sui event worker.
    workdir_idx: WorkdirIdx,
}

impl WorkdirContext for PackagesPollerParams {
    fn workdir_idx(&self) -> WorkdirIdx {
        self.workdir_idx
    }
}

impl PackagesPollerParams {
    pub fn new(
        globals: Globals,
        sui_events_worker_tx: Option<GenericTx>,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            globals,
            sui_events_worker_tx,
            workdir_idx,
        }
    }
}

pub struct PackagesPoller {
    // "Glue" the specialized PollingTraitObject with its parameters.
    // The worker does all the background task/events handling.
    poller: PollerWorker<PollingTraitObject, PackagesPollerParams>,
}

pub struct PollingTraitObject {
    params: PackagesPollerParams,
}

#[async_trait]
impl PollingTrait for PollingTraitObject {
    // This is called by the PollerWorker task.
    async fn update(&mut self) {
        self.update_globals_workdir_packages().await;
    }
}

// This allow the PollerWorker to instantiate the PollingTraitObject.
impl Instantiable<PackagesPollerParams> for PollingTraitObject {
    fn new(params: PackagesPollerParams) -> Self {
        Self { params }
    }
}

impl PackagesPoller {
    pub fn new(params: PackagesPollerParams, subsys: &SubsystemHandle) -> Self {
        let poller =
            PollerWorker::<PollingTraitObject, PackagesPollerParams>::new(params.clone(), subsys);
        Self { poller }
    }

    pub fn get_tx_channel(&self) -> GenericTx {
        self.poller.get_tx_channel()
    }
}

#[allow(dead_code)]
struct PackagesPollerWorkerTask {
    task_name: String,
    params: PackagesPollerParams,
}

impl PollingTraitObject {
    async fn create_package_instance(
        &self,
        package_path: PackagePath,
        published_data_path: &PathBuf,
    ) -> Result<PackageInstance> {
        // Read the package_id from the filesystem in the file
        // package_path.get_path()/package-id.json
        // Example of package-id.json:
        //     ["0x85d7bf998ba94d55f3f143f1415edf7cebe3d67efcd9550d541b929ef3f9c693"]

        let package_id_path = package_path
            .get_path(published_data_path)
            .join("package-id.json");

        let package_id = tokio::fs::read_to_string(&package_id_path)
            .await
            .map_err(|e| anyhow!("Failed to read {}: {}", package_id_path.display(), e))?;

        // Validate that package_id to be ["0x<64_hex_digits>"]
        let package_id = package_id.trim();
        if package_id.len() < 30 || !package_id.starts_with("[\"0x") || !package_id.ends_with("\"]")
        {
            return Err(anyhow!(
                "Invalid package_id {} in {}",
                package_id,
                package_id_path.display()
            ));
        }
        // Remove the ["0x and the last "]
        let package_id = &package_id[4..package_id.len() - 2];

        // Load package_path.get_path()/created-objects.json
        //
        // Example of created-objects.json:
        // [{"objectId":"0x3a434796fb233dfca274c31c58cb26072aedbe20ecd4a674c399504d6106a29c","type":"0x2::package::UpgradeCap"},
        //  {"objectId":"0x511a9a507f89cae38d4ea97089f314b7f29e39160c83f1d3d47631925e6ead7b","type":"0x85d7bf998ba94d55f3f143f1415edf7cebe3d67efcd9550d541b929ef3f9c693::logger::Logger"},
        //  {"objectId":"0x60f36fcedd3dd6c1194ce2c5fa1ce0baa75f07e1d6cadf68ebf80aea04483f8a","type":"0x85d7bf998ba94d55f3f143f1415edf7cebe3d67efcd9550d541b929ef3f9c693::Counter::Counter"},
        //  {"objectId":"0x7192c01109802e1d37420275183f275aa5d0e4a7037184c3119d3f2e56293acc","type":"0x85d7bf998ba94d55f3f143f1415edf7cebe3d67efcd9550d541b929ef3f9c693::logger_admin_cap::LoggerAdminCap"}
        // ]
        //
        let mut objects: Vec<SuiObjectInstance> = Vec::new();
        let file_content = tokio::fs::read_to_string(
            package_path
                .get_path(published_data_path)
                .join("created-objects.json"),
        )
        .await?;

        let top: serde_json::Value = serde_json::from_str(&file_content)?;

        if let Some(top_array) = top.as_array() {
            for created_object in top_array {
                if let Some(type_field) = created_object.get("type") {
                    if let Some(type_str) = type_field.as_str() {
                        let substrings: Vec<&str> = type_str.split("::").collect();
                        if substrings.len() == 3 {
                            if let Some(objectid_field) = created_object.get("objectId") {
                                if let Some(objectid_str) = objectid_field.as_str() {
                                    let file_pid = substrings[0].to_string();
                                    // Remove leading 0x if any.
                                    let file_pid = file_pid.trim_start_matches("0x");
                                    let ui_pid = if file_pid == package_id {
                                        None
                                    } else {
                                        Some(file_pid.to_string())
                                    };
                                    let object_type = SuiObjectType::new(
                                        ui_pid,
                                        substrings[1].to_string(),
                                        substrings[2].to_string(),
                                    );
                                    objects.push(SuiObjectInstance::new(
                                        objectid_str.to_string(),
                                        Some(object_type),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut ret_value = PackageInstance::new(package_id.to_string(), package_path);
        ret_value.set_init_objects(objects);
        // TODO ret_value.set_package_owner
        Ok(ret_value)
    }

    async fn update_globals_workdir_packages(&mut self) {
        let workdir_idx = self.params.workdir_idx;
        let workdir = WORKDIRS_KEYS[workdir_idx as usize].to_string();

        // Multiple steps for efficiency:
        // Step 1) Read the Filesystem to get all the published PackagePath.
        //         That means 3 loops to iterate package_name, package_uuid
        //         and package_timestamp directory. Build the all_published_packages Vec.
        //
        // Step 2) With global read lock:
        //        - Put in "to_be_removed" the packages in globals, but not on filesystem.
        //        - Put in "to_be_added" the packages not in globals.
        //
        // Step 3) Create a PackageInstance for every UUID in "to_be_added" (this may
        //     involve further filesystem reading)
        // Step 4) With global write lock, apply to_be_added and to_be_removed changes.
        //

        // Step 1
        let published_data_path = match self.get_published_data_path(workdir_idx).await {
            Ok(published_data_path) => published_data_path,
            Err(e) => {
                let err_msg = format!("{} {}", workdir, e);
                log::error!("{}", err_msg);
                return;
            }
        };

        let all_published_packages: HashSet<PackagePath> =
            match Self::get_all_published_packages(published_data_path.clone()).await {
                Ok(packages) => packages,
                Err(e) => {
                    // This is not an error if the directory does not exists. It just means
                    // there is no published packages yet. Just return an empty HashSet.
                    let metadata = tokio::fs::metadata(&published_data_path).await;

                    match metadata {
                        Ok(_metadata) => {
                            // The path exists... but reading failed.
                            let err_msg =
                                format!("Failed to get all {} published packages: {}", workdir, e);
                            log_safe!(err_msg);
                            return;
                        }
                        Err(_e) => {
                            HashSet::new() // No package yet? Return an empty HashSet
                        }
                    }
                }
            };

        // Step 2
        // TODO Limit package instances in UI + package instance tagging to preserve in UI.
        let mut to_be_removed: Vec<PackagePath> = Vec::new();
        let mut to_be_added: Vec<PackagePath> = Vec::new();
        let no_change_resp_header = {
            let globals_read_guard = self.params.globals.get_packages(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                let wp_resp = ui.get_data();

                for package_path in &all_published_packages {
                    if !wp_resp.contains(package_path) {
                        to_be_added.push(package_path.clone());
                    }
                }

                let global_package_count = wp_resp.package_count();
                if all_published_packages.is_empty() && global_package_count > 0 {
                    // Remove them all at once!
                    to_be_removed.extend(wp_resp.iter_package_paths().cloned());
                } else if global_package_count > all_published_packages.len() {
                    // Only remove the extra ones.
                    for package_path in wp_resp.iter_package_paths() {
                        if !all_published_packages.contains(package_path) {
                            to_be_removed.push(package_path.clone());
                        }
                    }
                }

                true
            } else {
                false
            }
        };

        // Step 3.
        let mut to_be_added_packages: Vec<PackageInstance> = Vec::new();
        for package_path in to_be_added {
            // Convert the PackagePath into a PackageInstance (some I/O will happen).
            // Just ignore on any I/O error.
            match self
                .create_package_instance(package_path, &published_data_path)
                .await
            {
                Ok(package_instance) => to_be_added_packages.push(package_instance),
                Err(e) => {
                    log_safe!(format!(
                        "Failed to create {} package instance (2): {}",
                        workdir, e
                    ));
                }
            }
        }

        if to_be_added_packages.is_empty() && to_be_removed.is_empty() && no_change_resp_header {
            // No change needed to UI.
            return;
        }

        // Merge to_be_added_packages with globals and create a new resp as needed.
        // Also remove to_be_removed from globals.
        // This is a write lock on the globals.
        let mut at_least_one_ui_change = false;
        {
            let mut globals_write_guard =
                self.params.globals.get_packages(workdir_idx).write().await;
            let globals = &mut *globals_write_guard;

            // Note: Keep in mind that between the read and this write lock, the globals may have already
            //       changed, but to_be_added_packages and to_be_removed are applied regardless... and
            //       it is assumed the globals will eventually converge to the correct state.
            if globals.ui.is_none() {
                globals.init_empty_ui(workdir.clone());
            }

            if let Some(ui) = &mut globals.ui {
                if !to_be_removed.is_empty() {
                    // Iterate to_be_removed and remove each from globals.ui
                    let wp_resp = ui.get_mut_data();

                    for package_path in &to_be_removed {
                        if wp_resp.delete_package_instance(package_path) {
                            at_least_one_ui_change = true;
                        }
                    }
                }
                if !to_be_added_packages.is_empty() {
                    let wp_resp = ui.get_mut_data();
                    for package_instance in to_be_added_packages {
                        if wp_resp.add_package_instance(package_instance, None) {
                            at_least_one_ui_change = true;
                        }
                    }
                }
                if at_least_one_ui_change {
                    ui.inc_uuid();
                }
            }
        }

        if at_least_one_ui_change {
            if let Some(sui_events_worker_tx) = &self.params.sui_events_worker_tx {
                // Send an internal message to have the events Sui workers do the package
                // tracking sooner than (periodically) later.
                let mut msg = GenericChannelMsg::new();
                msg.event_id = basic_types::EVENT_UPDATE;
                msg.workdir_idx = Some(workdir_idx);
                if let Err(e) = sui_events_worker_tx.try_send(msg) {
                    let err_msg = format!(
                        "try_send {} EVENT_UPDATE to events_worker failed: {}",
                        workdir, e
                    );
                    log_safe!(err_msg);
                }
            }
        }
    }

    async fn get_published_data_path(&self, workdir_idx: WorkdirIdx) -> Result<PathBuf> {
        let workdir_paths = common::shared_types::get_workdir_paths(workdir_idx);
        let workdir_path = workdir_paths.workdir_root_path();

        // The package_uuid is a string.
        Ok(workdir_path.join("published-data"))
    }

    async fn get_all_published_packages(
        published_data_path: PathBuf,
    ) -> Result<HashSet<PackagePath>> {
        let mut all_published_packages = HashSet::new();
        let published_data_path = published_data_path.clone();
        let published_data_path = published_data_path.as_path();
        let mut entries = tokio::fs::read_dir(published_data_path)
            .await
            .map_err(|err| {
                anyhow!(
                    "Failed to read_dir [{}]: {}",
                    published_data_path.display(),
                    err
                )
            })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let file_name = path.file_name().ok_or_else(|| anyhow!("No file name"))?;
                let package_name = file_name
                    .to_str()
                    .ok_or_else(|| anyhow!("Invalid Unicode"))?
                    .to_string();
                let mut entries = tokio::fs::read_dir(path).await?;

                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.is_dir() {
                        let file_name = path.file_name().ok_or_else(|| anyhow!("No file name"))?;
                        let package_uuid = file_name
                            .to_str()
                            .ok_or_else(|| anyhow!("Invalid Unicode"))?
                            .to_string();
                        // Skip invalid pacakge_uuid directories.
                        // (could be normal, like "most-recent" soft link)
                        if !PackagePath::is_valid_package_uuid(&package_uuid) {
                            continue;
                        }
                        let mut entries = tokio::fs::read_dir(path).await?;

                        while let Some(entry) = entries.next_entry().await? {
                            let path = entry.path();
                            if path.is_dir() {
                                let file_name =
                                    path.file_name().ok_or_else(|| anyhow!("No file name"))?;
                                let package_timestamp = file_name
                                    .to_str()
                                    .ok_or_else(|| anyhow!("Invalid Unicode"))?
                                    .to_string();
                                // Skip invalid package_timestamp directories (contains non-numeric characters).
                                // (could be normal, like "most-recent-timestamp" soft link)
                                if !PackagePath::is_valid_package_timestamp(&package_timestamp) {
                                    continue;
                                }
                                // Quick validation that this directory contains:
                                //   package-id.json, created-objects.json and publish-output.json
                                let package_id_path = path.join("package-id.json");
                                let created_objects_path = path.join("created-objects.json");
                                let publish_output_path = path.join("publish-output.json");
                                if tokio::fs::metadata(&package_id_path).await.is_err()
                                    || tokio::fs::metadata(&created_objects_path).await.is_err()
                                    || tokio::fs::metadata(&publish_output_path).await.is_err()
                                {
                                    continue;
                                }
                                all_published_packages.insert(PackagePath::new(
                                    package_name.clone(),
                                    package_uuid.clone(),
                                    package_timestamp.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(all_published_packages)
    }

    // Done after a successful "publish" CLI command.
    //
    // Intent is for the backend to react ASAP for event registering of the latest.
    //
    // If this is somehow missed, the next audit will catch it.
    /*
    async fn update_globals_post_publish(&self, package_instance: PackageInstance) {
        let workdir_idx = self.params.workdir_idx;
        let mut globals_write_guard = self.params.globals.get_packages(workdir_idx).write().await;
        let globals = &mut *globals_write_guard;

        if globals.ui.is_none() {
            globals.init_empty_ui(self.params.workdir_name.clone());
        }

        if let Some(ui) = &mut globals.ui {
            let wp_resp = ui.get_mut_data();
            wp_resp.add_package_instance(package_instance, None);
            ui.inc_uuid();
        }
    }*/
}
