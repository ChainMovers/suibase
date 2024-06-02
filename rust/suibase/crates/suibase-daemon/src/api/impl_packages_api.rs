use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

use axum::async_trait;

use anyhow::Result;

use common::basic_types::WorkdirIdx;
use common::log_safe;
use jsonrpsee::core::RpcResult;
use jsonrpsee_types::ErrorObjectOwned as RpcError;

use chrono::Utc;

use crate::admin_controller::{AdminController, AdminControllerTx};

use crate::api::RpcSuibaseError;
use crate::shared_types::{Globals, GlobalsWorkdirsST, PackagePath};
use anyhow::anyhow;

use super::{
    Header, PackageInstance, PackagesApiServer, RpcInputError, SuccessResponse, SuiObjectInstance,
    SuiObjectType, WorkdirPackagesResponse, WorkdirSuiEventsResponse,
};

pub struct PackagesApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
}

impl PackagesApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }

    // Utility function to generate hash for the move_toml_path
    // and return it as a string.
    pub fn short_hash(move_toml_path: &str) -> String {
        // the string is a RFC4648 Base32 (no pad) of the md5sum of the move_toml_path.
        let md5 = md5::compute(move_toml_path);
        data_encoding::BASE32_NOPAD.encode(&md5.to_vec())
    }
}

#[async_trait]
impl PackagesApiServer for PackagesApiImpl {
    async fn get_workdir_events(
        &self,
        workdir: String,
        _after_ts: Option<String>,
        _last_ts: Option<String>,
    ) -> RpcResult<WorkdirSuiEventsResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.
        //

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let _workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Initialize some of the header fields of the response.
        let mut resp = WorkdirSuiEventsResponse::new();
        resp.header.method = "getEvents".to_string();
        resp.header.key = Some(workdir.clone());
        Ok(resp)
    }

    // Called prior to a network publication.
    //
    // Returns the package_uuid to be used for the specified package.
    async fn pre_publish(
        &self,
        workdir: String,
        move_toml_path: String,
        package_name: String,
    ) -> RpcResult<SuccessResponse> {
        // Initialize some of the header fields of the response.
        let mut resp = SuccessResponse::new();
        resp.header.method = "prePublish".to_string();

        match self
            .internal_prepublish(&workdir, &move_toml_path, &package_name)
            .await
        {
            Ok((_workdir_idx, package_uuid)) => {
                let now = SystemTime::now();
                let package_timestamp = now
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                resp.result = true;
                resp.info = Some(format!("{},{}", package_uuid, package_timestamp));
                Ok(resp)
            }
            Err(e) => Err(e),
        }
    }

    // Called after a network publication.
    //
    // This is mainly for reporting the new package_id to the daemon.
    async fn post_publish(
        &self,
        workdir: String,
        move_toml_path: String,
        package_name: String,
        package_uuid: String,
        package_timestamp: String,
        _package_id: String,
    ) -> RpcResult<SuccessResponse> {
        // TODO More parameters validation.
        log_safe!(format!("post_publish: workdir={}, move_toml_path={}, package_name={}, package_uuid={}, package_timestamp={}",
            workdir, move_toml_path, package_name, package_uuid, package_timestamp));

        // Initialize some of the header fields of the response.
        let mut resp = SuccessResponse::new();
        resp.header.method = "postPublish".to_string();

        // Run prepublish again to validate the package_uuid provided is
        // consistent with what is written on the filesystem. If not,
        // there must be some race condition. Fail the publication
        // to bring this to user attention.

        let prepublish = self
            .internal_prepublish(&workdir, &move_toml_path, &package_name)
            .await;
        let (workdir_idx, fs_package_uuid) = match prepublish {
            Ok((workdir_idx, fs_package_uuid)) => (workdir_idx, fs_package_uuid),
            Err(e) => {
                return Err(e);
            }
        };
        if package_uuid != fs_package_uuid {
            let err_msg = format!(
                "Possible race condition among concurrent publications. Try again (Reason: {} != {})",
                package_uuid, fs_package_uuid
            );
            log::error!("{}", err_msg);
            return Err(RpcSuibaseError::InternalError(err_msg).into());
        }

        // Remove any potential leading 0x to package_id.
        // .create_package_instance() reads the package-id.json file
        // let package_id = package_id.trim_start_matches("0x").to_string();

        // Create the PackageInstance.
        let published_data_path = match self.get_published_data_path(workdir_idx).await {
            Ok(published_data_path) => published_data_path,
            Err(e) => {
                let err_msg = format!("Failed to get published data path: {}", e);
                log::error!("{}", err_msg);
                return Err(RpcSuibaseError::InternalError(err_msg).into());
            }
        };

        let package_instance = match self
            .create_package_instance(
                PackagePath::new(package_name, package_uuid, package_timestamp),
                &published_data_path,
            )
            .await
        {
            Ok(package_instance) => package_instance,
            Err(e) => {
                let err_msg = format!("Failed to create package instance (1): {}", e);
                log_safe!(err_msg);
                return Err(RpcSuibaseError::InternalError(err_msg).into());
            }
        };

        // Insert the data in the globals.
        {
            let mut globals_write_guard = self.globals.get_packages(workdir_idx).write().await;
            let globals = &mut *globals_write_guard;

            // Create the globals.ui if does not exists.
            if globals.ui.is_none() {
                globals.init_empty_ui(workdir.clone());
            }

            if let Some(ui) = &mut globals.ui {
                let wp_resp = ui.get_mut_data();
                wp_resp.add_package_instance(package_instance, Some(move_toml_path));

                // Always bump the UUIDs.
                ui.inc_uuid();
                ui.write_uuids_into_header_param(&mut resp.header);
            }
        }

        // The writer lock on global is now released. Send an internal message to have
        // the websocket workers do the package tracking.
        if AdminController::send_event_audit(&self.admctrl_tx)
            .await
            .is_err()
        {
            let err_msg = "Failed to send event audit to admin controller".to_string();
            log::error!("{}", err_msg);
            return Err(RpcSuibaseError::InternalError(err_msg).into());
        }

        // Return success.
        resp.header.key = Some(workdir.clone());
        resp.result = true;
        Ok(resp)
    }

    async fn get_workdir_packages(
        &self,
        workdir: String,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<WorkdirPackagesResponse> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        if method_uuid.is_none() && data_uuid.is_none() {
            // Best-effort refresh, since user is requesting for the latest.

            // Allow only one API request for a given workdir at the time to avoid race conditions.
            let mut api_mutex_guard = self.globals.get_api_mutex(workdir_idx).lock().await;
            let api_mutex = &mut *api_mutex_guard;

            let last_api_call_timestamp = &mut api_mutex.last_get_workdir_status_time;

            let _ = self
                .update_globals_workdir_packages(
                    workdir.clone(),
                    workdir_idx,
                    last_api_call_timestamp,
                )
                .await;
        }

        {
            let globals_read_guard = self.globals.get_packages(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                if let (Some(method_uuid), Some(data_uuid)) = (method_uuid, data_uuid) {
                    let mut are_same_version = false;
                    let globals_data_uuid = ui.get_uuid().get_data_uuid();
                    if data_uuid == globals_data_uuid {
                        let globals_method_uuid = ui.get_uuid().get_method_uuid();
                        if method_uuid == globals_method_uuid {
                            are_same_version = true;
                        }
                    }
                    if !are_same_version {
                        // Something went wrong, but this could be normal if the globals just got updated
                        // and the caller is not yet aware of it (assume the caller will eventually discover
                        // the latest version with getVersions).
                        return Err(RpcSuibaseError::OutdatedUUID().into());
                    }
                }
                // Response with the latest global data.
                let mut resp = ui.get_data().clone();
                resp.header.set_from_uuids(ui.get_uuid());
                return Ok(resp);
            } else {
                return Err(
                    RpcSuibaseError::InternalError("globals.ui was None".to_string()).into(),
                );
            }
        }
    }
}

impl PackagesApiImpl {
    async fn internal_prepublish(
        &self,
        workdir: &String,
        move_toml_path: &str,
        package_name: &String,
    ) -> Result<(u8, String), RpcError> {
        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::get_workdir_idx_by_name(&self.globals, workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => {
                return Err(
                    RpcInputError::InvalidParams("workdir".to_string(), workdir.clone()).into(),
                )
            }
        };

        // Identify the path of Suibase.toml co-located to the Move.toml
        let move_toml_path_only = move_toml_path.trim_end_matches("Move.toml");
        let toml_path = std::path::PathBuf::from(move_toml_path_only);
        let suibase_toml_path = toml_path.join("Suibase.toml");

        // if the file suibase_toml_path does not exists, then create it with the following content:
        //
        // [meta]
        // creation_timestamp = "<epoch timestamp in microseconds> ISO 8061 datetime in local timezone"
        //
        // [packages]
        // <package_name> = { uuid="<hash_of_move_toml_filepath>", custom_uuid=false }
        //
        // For now, it seems Mysten Labs will have only *one* package per Move.toml, so same
        // to be expected for the co-located Suibase.toml.
        //
        // Hash is defined as the RFC4648 Base32 (no pad) of the md5 bytes of the move_toml_path.
        let suibase_toml_string = if !suibase_toml_path.exists() {
            // Create the Suibase.toml file.
            let mut packages_section = toml_edit::Table::new();
            // Build a Table for each package.
            let mut package_table = toml_edit::InlineTable::new();
            package_table.insert("uuid", Self::short_hash(move_toml_path).into());
            let uuid_custom = false;
            package_table.insert("uuid_custom", uuid_custom.into());
            packages_section.insert(package_name, toml_edit::Item::Value(package_table.into()));

            let mut meta_section = toml_edit::Table::new();
            let now = std::time::SystemTime::now();
            let datetime_utc: chrono::DateTime<Utc> = now.into();
            let datetime_local = datetime_utc.with_timezone(&chrono::Local);
            meta_section.insert(
                "creation_timestamp",
                toml_edit::value(format!(
                    "{} {}",
                    datetime_utc.timestamp_micros(),
                    datetime_local
                )),
            );

            let mut suibase_toml_doc = toml_edit::Document::new();
            suibase_toml_doc["meta"] = toml_edit::Item::Table(meta_section);
            suibase_toml_doc["packages"] = toml_edit::Item::Table(packages_section);

            let new_file_string = suibase_toml_doc.to_string();
            match tokio::fs::write(suibase_toml_path.clone(), new_file_string.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    let err_msg = format!("Failed to write Suibase.toml: {}", e);
                    log::error!("{}", err_msg);
                    return Err(RpcSuibaseError::FileAccessError(err_msg).into());
                }
            }
            new_file_string
        } else {
            // Read the existing Suibase.toml file.
            match tokio::fs::read_to_string(suibase_toml_path.clone()).await {
                Ok(read_string) => read_string,
                Err(e) => {
                    let err_msg = format!("Failed to read Suibase.toml: {}", e);
                    log::error!("{}", err_msg);
                    return Err(RpcSuibaseError::FileAccessError(err_msg).into());
                }
            }
        };

        // TODO Add robustness to toml_edit if the document exists but the user deleted the UUID field.
        // TODO Implement to regenerate the UUID if moving of Suibase.toml is detected. Need to put path in toml for this.

        let suibase_toml_doc = match suibase_toml_string.parse::<toml_edit::Document>() {
            Ok(suibase_toml_doc) => Some(suibase_toml_doc),
            Err(e) => {
                log::error!("Failed to parse Suibase.toml: {}", e);
                None
            }
        };

        // TODO: Handling of multiple package.
        let mut package_uuid = String::new();
        if let Some(doc) = suibase_toml_doc {
            if let Some(packages) = doc.get("packages") {
                if let Some(packages) = packages.as_table() {
                    // Iterate the packages until finding the one that match package_name.
                    for (key, value) in packages.iter() {
                        if *key == *package_name {
                            if let Some(value) = value.as_inline_table() {
                                if let Some(uuid) = value.get("uuid") {
                                    if let Some(uuid) = uuid.as_str() {
                                        package_uuid = uuid.to_string();
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        // If somehow could not load the package_uuid from the file, then default
        // to a calculated value.

        // TODO: Try to fix the Suibase.toml and try once again.
        if package_uuid.is_empty() {
            package_uuid = Self::short_hash(move_toml_path);
        }

        Ok((workdir_idx, package_uuid))
    }

    async fn get_all_published_packages(
        published_data_path: PathBuf,
    ) -> Result<HashSet<PackagePath>> {
        let mut all_published_packages = HashSet::new();
        let published_data_path = published_data_path.clone();
        let published_data_path = published_data_path.as_path();
        let mut entries = tokio::fs::read_dir(published_data_path).await?;

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
                                if !tokio::fs::metadata(&package_id_path).await.is_ok()
                                    || !tokio::fs::metadata(&created_objects_path).await.is_ok()
                                    || !tokio::fs::metadata(&publish_output_path).await.is_ok()
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

    async fn update_globals_workdir_packages(
        &self,
        workdir: String,
        workdir_idx: WorkdirIdx,
        last_api_call_timestamp: &mut tokio::time::Instant,
    ) -> Result<Header> {
        // Debounce excessive refresh request on short period of time.
        if last_api_call_timestamp.elapsed() < tokio::time::Duration::from_millis(50) {
            let globals_read_guard = self.globals.get_packages(workdir_idx).read().await;
            let globals = &*globals_read_guard;

            if let Some(ui) = &globals.ui {
                return Ok(ui.get_data().header.clone());
            }
        };
        *last_api_call_timestamp = tokio::time::Instant::now();

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
            Err(e) => return Err(anyhow!("{} {} ", workdir, e.to_string())),
        };

        let all_published_packages: HashSet<PackagePath> =
            match Self::get_all_published_packages(published_data_path.clone()).await {
                Ok(packages) => packages,
                Err(e) => {
                    let err_msg = format!("Failed to get all published packages: {}", e);
                    log::error!("{}", err_msg);
                    return Err(RpcSuibaseError::InternalError(err_msg).into());
                }
            };

        // Step 2
        // TODO Limit package instances in UI + package instance tagging to preserve in UI.
        let mut to_be_removed: Vec<PackagePath> = Vec::new();
        let mut to_be_added: Vec<PackagePath> = Vec::new();
        let no_change_resp_header = {
            let globals_read_guard = self.globals.get_packages(workdir_idx).read().await;
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

                Some(ui.get_data().header.clone())
            } else {
                None
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
                    log_safe!(format!("Failed to create package instance (2): {}", e));
                }
            }
        }

        if to_be_added_packages.is_empty() && to_be_removed.is_empty() {
            if let Some(no_change_resp_header) = no_change_resp_header {
                // No change needed to UI.
                return Ok(no_change_resp_header);
            }
        }

        // Merge to_be_added_packages with globals and create a new resp as needed.
        // Also remove to_be_removed from globals.
        // This is a write lock on the globals.
        let resp_header = {
            let mut globals_write_guard = self.globals.get_packages(workdir_idx).write().await;
            let globals = &mut *globals_write_guard;

            // Note: Keep in mind that between the read and this write lock, the globals may have already
            //       changed, but to_be_added_packages and to_be_removed are applied regardless... and
            //       it is assumed the globals will eventually converge to the correct state.
            if globals.ui.is_none() {
                globals.init_empty_ui(workdir.clone());
            }

            if let Some(ui) = &mut globals.ui {
                let mut at_least_one_ui_change = false;
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

            globals.ui.as_ref().unwrap().get_data().header.clone()
        };

        Ok(resp_header)
    }

    async fn get_published_data_path(&self, workdir_idx: WorkdirIdx) -> Result<PathBuf> {
        let workdir_path = {
            let workdirs_guard = self.globals.workdirs.read().await;
            let workdirs = &*workdirs_guard;
            let workdir = workdirs
                .get_workdir(workdir_idx)
                .ok_or_else(|| anyhow!("Failed to get workdir by index {}", workdir_idx))?;
            workdir.path_cloned()
        };

        // The package_uuid is a string.
        Ok(workdir_path.join("published-data"))
    }
}
