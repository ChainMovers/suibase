use std::collections::HashMap;

use axum::async_trait;

use jsonrpsee::core::RpcResult;

use chrono::Utc;

use crate::admin_controller::AdminControllerTx;
use crate::shared_types::{Globals, GlobalsWorkdirsST};

use super::{
    MoveConfig, PackageInstance, PackagesApiServer, PackagesConfigResponse, RpcInputError,
    SuccessResponse, SuiEventsResponse,
};

use super::def_header::Versioned;

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
    async fn get_events(
        &self,
        workdir: String,
        _after_ts: Option<String>,
        _last_ts: Option<String>,
    ) -> RpcResult<SuiEventsResponse> {
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //
        // They all default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.
        //

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let _workdir_idx =
            match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir).await {
                Some(workdir_idx) => workdir_idx,
                None => {
                    return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into())
                }
            };

        // Initialize some of the header fields of the response.
        let mut resp = SuiEventsResponse::new();
        resp.header.method = "getEvents".to_string();
        resp.header.key = Some(workdir.clone());
        Ok(resp)
    }

    async fn publish(
        &self,
        workdir: String,
        move_toml_path: String,
        package_id: String,
        package_name: String,
    ) -> RpcResult<SuccessResponse> {
        // TODO More parameters validation.

        // Initialize some of the header fields of the response.
        let mut resp = SuccessResponse::new();
        resp.header.method = "publish".to_string();

        // Verify workdir param is OK and get its corresponding workdir_idx.
        let workdir_idx = match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
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
        // <package_name> = { output_dir="<hash_of_move_toml_filepath>", output_dir_custom=false }
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
            package_table.insert("output_dir", Self::short_hash(&move_toml_path).into());
            let output_dir_custom = false;
            package_table.insert("output_dir_custom", output_dir_custom.into());
            packages_section.insert(&package_name, toml_edit::Item::Value(package_table.into()));

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
                    resp.success = false;
                    resp.info = Some(err_msg);
                    return Ok(resp);
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
                    resp.success = false;
                    resp.info = Some(err_msg);
                    return Ok(resp);
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
                        if *key == package_name {
                            if let Some(value) = value.as_inline_table() {
                                if let Some(output_dir) = value.get("output_dir") {
                                    if let Some(output_dir) = output_dir.as_str() {
                                        package_uuid = output_dir.to_string();
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
        if package_uuid.is_empty() {
            package_uuid = Self::short_hash(&move_toml_path);
        }

        // Insert the data in the globals.
        {
            let mut globals_write_guard = self.globals.packages_config.write().await;
            let globals = &mut *globals_write_guard;
            let globals = globals.workdirs.get_mut(workdir_idx);

            if globals.ui.is_none() {
                globals.ui = Some(Versioned::new(PackagesConfigResponse::new()));
            }
            let ui = globals.ui.as_mut().unwrap();

            let config_resp = ui.get_mut_data();

            if config_resp.move_configs.is_none() {
                config_resp.move_configs = Some(HashMap::new());
            }
            let move_configs = config_resp.move_configs.as_mut().unwrap();
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
                    resp.success = true;
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
            ));

            // Make sure the latest known path is correctly reflected in globals.
            if move_config.path.is_none() || (move_config.path.as_ref().unwrap() != &move_toml_path)
            {
                move_config.path = Some(move_toml_path.clone());
            }

            // TODO Create init_objects by parsing the JSON output.
        }

        // Return success.
        resp.header.key = Some(workdir.clone());
        resp.success = true;
        Ok(resp)
    }

    async fn get_packages_config(
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
        let workdir_idx = match GlobalsWorkdirsST::find_workdir_idx_by_name(&self.globals, &workdir)
            .await
        {
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
                                ui.init_header_uuids(&mut resp.header);
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
