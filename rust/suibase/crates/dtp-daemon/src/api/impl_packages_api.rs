use std::time::SystemTime;

use axum::async_trait;

use jsonrpsee::core::RpcResult;
use jsonrpsee_types::ErrorObjectOwned as RpcError;

use chrono::Utc;

use crate::admin_controller::{AdminController, AdminControllerTx};
use crate::api::RpcSuibaseError;
use crate::shared_types::{Globals, GlobalsPackagesConfigST};

use super::{
    MoveConfig, PackageInstance, PackagesApiServer, PackagesConfigResponse, RpcInputError,
    SuccessResponse, WorkdirSuiEventsResponse,
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
        let _workdir_idx = match self.globals.get_workdir_idx_by_name(&workdir).await {
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
        package_id: String,
    ) -> RpcResult<SuccessResponse> {
        // TODO More parameters validation.

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
        let package_id = package_id.trim_start_matches("0x").to_string();

        // Insert the data in the globals.
        {
            let mut globals_write_guard = self.globals.packages_config.write().await;
            let globals = &mut *globals_write_guard;

            let move_configs =
                GlobalsPackagesConfigST::get_mut_move_configs(&mut globals.workdirs, workdir_idx);

            let mut move_config = move_configs.get_mut(&package_uuid);
            if move_config.is_none() {
                // Delete any other move_configs element where path equals move_toml_path.
                move_configs.retain(|_, config| {
                    if let Some(path) = &config.path {
                        if path == &move_toml_path {
                            return false;
                        }
                    }
                    true
                });

                let mut new_move_config = MoveConfig::new();
                new_move_config.path = Some(move_toml_path.clone());
                move_configs.insert(package_uuid.clone(), new_move_config);
                move_config = Some(move_configs.get_mut(&package_uuid).unwrap());
            }
            let move_config = move_config.unwrap();

            if let Some(current_package) = move_config.latest_package.take() {
                if current_package.package_id == package_id {
                    // This package is already the latest. Ignore this redundant publish request.
                    move_config.latest_package = Some(current_package); // Put it back.
                    resp.result = true;
                    resp.info = Some("Package is already the current one.".to_string());
                    return Ok(resp);
                }

                // Move current package into the list of previous packages.
                move_config.older_packages.push(current_package);
            }

            // Initialize this new current package.
            move_config.latest_package = Some(PackageInstance::new(
                package_id.clone(),
                package_name.clone(),
                package_timestamp.clone(),
            ));

            // Make sure the latest known path is correctly reflected in globals.
            if move_config.path.is_none() || (move_config.path.as_ref().unwrap() != &move_toml_path)
            {
                move_config.path = Some(move_toml_path.clone());
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

    async fn get_workdir_packages_config(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
        method_uuid: Option<String>,
        data_uuid: Option<String>,
    ) -> RpcResult<PackagesConfigResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.

        // TODO Implement display/debug requests.
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let _data = data.unwrap_or(!(debug || display));

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match self.globals.get_workdir_idx_by_name(&workdir).await {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        let mut resp_ready: Option<PackagesConfigResponse> = None;

        // Just return what is already built in-memory, or empty.
        {
            // Get the globals for the target workdir_idx.
            let globals_read_guard = self.globals.packages_config.read().await;
            let globals = &*globals_read_guard;
            let globals = globals.workdirs.get_if_some(workdir_idx);

            if let Some(globals) = globals {
                if let Some(ui) = &globals.ui {
                    if let (Some(method_uuid), Some(data_uuid)) = (method_uuid, data_uuid) {
                        let globals_data_uuid = ui.get_uuid().get_data_uuid();
                        if data_uuid == globals_data_uuid {
                            let globals_method_uuid = ui.get_uuid().get_method_uuid();
                            if method_uuid == globals_method_uuid {
                                // The caller requested the same data that it already have a copy of.
                                // Respond with the same UUID as a way to say "no change".
                                let mut resp = PackagesConfigResponse::new();
                                ui.write_uuids_into_header_param(&mut resp.header);
                                resp_ready = Some(resp);
                            }
                        }
                    } else {
                        // The caller did not specify a method_uuid or data_uuid and
                        // there is an in-memory response ready. Just respond with it.
                        resp_ready = Some(ui.get_data().clone());
                    }
                }
            }
        }

        if resp_ready.is_none() {
            resp_ready = Some(PackagesConfigResponse::new());
        }
        let mut resp_ready = resp_ready.unwrap();

        resp_ready.header.method = "getPackagesConfig".to_string();
        resp_ready.header.key = Some(workdir.clone());
        return Ok(resp_ready);
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
        let workdir_idx = match self.globals.get_workdir_idx_by_name(workdir).await {
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
}
