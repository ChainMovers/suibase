use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

use serde_json::Value;

use sui_sdk::types::base_types::{ObjectID, SuiAddress};

use crate::sui_base_root::SuiBaseRoot;

use anyhow::{bail, ensure, Context};

pub(crate) struct SuiBaseWorkdir {
    workdir_name: Option<String>,
    workdir_path: Option<String>,
}

impl SuiBaseWorkdir {
    pub fn new() -> SuiBaseWorkdir {
        SuiBaseWorkdir {
            workdir_name: None,
            workdir_path: None,
        }
    }

    pub(crate) fn init_from_existing(
        &mut self,
        root: &mut SuiBaseRoot,
        workdir_name: &str,
    ) -> Result<(), anyhow::Error> {
        ensure!(
            root.is_sui_base_installed(),
            "sui-base: not installed. Need to run ~/sui-base/install"
        );

        ensure!(
            !workdir_name.is_empty(),
            "sui-base: Invalid workdir name (empty string)"
        );

        // Check that the workdir do exists.
        let mut path_buf = PathBuf::from(root.workdirs_path());
        path_buf.push(workdir_name);
        path_buf = std::fs::canonicalize(path_buf).with_context(|| {
            format!("sui-base: path could not be access. Check sui-base is selecting a valid active workdir")})?;

        let workdir_path = path_buf.to_string_lossy().to_string();
        let path_exists = if workdir_path.is_empty() {
            false
        } else {
            Path::new(&workdir_path).exists()
        };

        ensure!(path_exists, "sui-base: path [{}] not found.", workdir_path);

        // Get the actual workdir name from the .state/name
        //
        // It resolved the workdir name when "active", but also, it generally provides
        // a sanity check that the workdir was created and is read accessible by this app.
        path_buf.push(".state");
        path_buf.push("name");
        let mut in_str = std::fs::read_to_string(&path_buf)
            .with_context(|| format!("sui-base: active workdir read of .state/name failed."))?;
        in_str = in_str.trim().to_string();
        ensure!(
            !in_str.is_empty(),
            "sui-base: active workdir .state/name not set. Try to 'update' the workdir"
        );

        self.workdir_name = Some(in_str);
        self.workdir_path = Some(workdir_path);
        Ok(())
    }

    pub(crate) fn get_name(&self) -> Result<String, anyhow::Error> {
        ensure!(
            self.workdir_name.is_some(),
            "sui-base: workdir name not set"
        );
        // Safe to unwrap, because is_some() checked and then use
        // to_string to make a copy to bubble up.
        Ok(self.workdir_name.as_ref().unwrap().to_string())
    }

    pub(crate) fn get_package_id(
        &self,
        root: &mut SuiBaseRoot,
        package_name: &str,
    ) -> Result<ObjectID, anyhow::Error> {
        let pathname =
            self.get_pathname_published_file(root, package_name, "package-id", "json")?;

        let mut in_str = std::fs::read_to_string(&pathname)
            .with_context(|| format!("sui-base: could not open published data [{}].", pathname))?;
        in_str = in_str.trim().to_string();

        // Simple parsing for a generated file expected to be: ["<hex string>"]
        ensure!(
            in_str.starts_with("[\"") && in_str.ends_with("\"]"),
            "sui-base: invalid package-id.json format"
        );
        let package_id_hex: &str = &in_str[2..in_str.len() - 2];

        // Parse the expected hex string.
        let package_id = ObjectID::from_hex_literal(package_id_hex)?;
        Ok(package_id)
    }

    pub(crate) fn get_keystore_pathname(
        &self,
        root: &mut SuiBaseRoot,
    ) -> Result<String, anyhow::Error> {
        ensure!(
            root.is_sui_base_installed(),
            "sui-base: not installed. Need to run ~/sui-base/install"
        );

        ensure!(self.workdir_path.is_some(), "sui-base: workdir not set");
        let workdir_path = self.workdir_path.as_ref().unwrap().to_string();

        let mut path_buf = PathBuf::from(workdir_path);
        path_buf.push("config");
        path_buf.push("sui");
        path_buf.set_extension("keystore");

        // Suggest to run the Sui client if keystore does not exists.
        let keystore_file = path_buf.to_string_lossy().to_string();
        let keystore_file_exists = if keystore_file.is_empty() {
            false
        } else {
            Path::new(&keystore_file).exists()
        };

        ensure!(
            keystore_file_exists,
            "sui-base: Not finding keystore [{}]. Run the sui client to create it?",
            keystore_file
        );

        Ok(keystore_file)
    }

    pub(crate) fn get_published_new_objects(
        &self,
        root: &mut SuiBaseRoot,
        object_type: &str,
    ) -> Result<Vec<ObjectID>, anyhow::Error> {
        // Validate the parameter format.
        let mut names = vec![];
        for found in object_type.split("::") {
            let trim_str = found.trim();
            // A name can't be whitespaces.
            ensure!(
                !trim_str.is_empty(),
                "sui-base: invalid object_type parameter with missing field"
            );
            names.push(trim_str);
        }
        ensure!(names.len() == 3, "sui-base: invalid object_type parameter");

        let pathname =
            self.get_pathname_published_file(root, names[0], "created-objects", "json")?;

        // Load the created-objects.json file.
        let file = File::open(pathname)?;
        let reader = BufReader::new(file);
        let top: Value = serde_json::from_reader(reader)?;

        let mut objects = vec![];

        if let Some(top_array) = top.as_array() {
            for created_object in top_array {
                if let Some(type_field) = created_object.get("type") {
                    if let Some(type_str) = type_field.as_str() {
                        let substrings: Vec<&str> = type_str.split("::").collect();
                        // TODO Check package id and name. substrings[0] != names[0] ||
                        if substrings.len() == 3
                            && substrings[1] == names[1]
                            && substrings[2] == names[2]
                        {
                            if let Some(objectid_field) = created_object.get("objectid") {
                                if let Some(objectid_str) = objectid_field.as_str() {
                                    objects.push(ObjectID::from_hex_literal(objectid_str)?);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(objects)
    }

    pub(crate) fn get_client_address(
        &self,
        root: &mut SuiBaseRoot,
        address_name: &str,
    ) -> Result<SuiAddress, anyhow::Error> {
        // Validate the parameters.
        ensure!(
            !address_name.is_empty(),
            "sui-base: invalid client_name parameter"
        );

        let pathname = self.get_pathname_state(root, "dns")?;

        // Load the dns file, which is a JSON file.
        let file = File::open(pathname)?;
        let reader = BufReader::new(file);
        let top: HashMap<String, Value> = serde_json::from_reader(reader)?;

        if let Some(known) = top.get("known") {
            if let Some(known_item) = known.get(address_name) {
                if let Some(address_v) = known_item.get("address") {
                    if let Some(address_str) = address_v.as_str() {
                        return SuiAddress::from_str(address_str);
                    }
                }
            }
        }

        bail!("sui-base: not finding client address [{}]", address_name);
    }

    pub(crate) fn get_rpc_url(&self, root: &mut SuiBaseRoot) -> Result<String, anyhow::Error> {
        self.get_url_from_state(root, "rpc")
    }

    pub(crate) fn get_ws_url(&self, root: &mut SuiBaseRoot) -> Result<String, anyhow::Error> {
        self.get_url_from_state(root, "ws")
    }
}

impl SuiBaseWorkdir {
    //*************************************************/
    // This scope is for the private utility functions.
    //*************************************************/
    fn get_pathname_published_file(
        &self,
        root: &mut SuiBaseRoot,
        package_name: &str,
        file_name: &str,
        extension: &str,
    ) -> Result<String, anyhow::Error> {
        // Build pathname and do some error detections.
        ensure!(
            root.is_sui_base_installed(),
            "sui-base: not installed. Need to run ~/sui-base/install"
        );

        ensure!(
            !package_name.is_empty(),
            "sui-base: Invalid package name (empty string)"
        );

        ensure!(
            self.workdir_name.is_some(),
            "sui-base: workdir name not set"
        );
        let workdir_name = self.workdir_name.as_ref().unwrap().to_string();

        ensure!(self.workdir_path.is_some(), "sui-base: workdir not set");
        let workdir_path = self.workdir_path.as_ref().unwrap().to_string();

        // Check if the publication of package was done.
        let mut path_buf = PathBuf::from(workdir_path);
        path_buf.push("published-data");
        path_buf.push(package_name);

        // Do an intermediate check to potentially error and provide a simplified advise
        // to publish the package (versus raising an error specific to package-id.json file).
        let published_path = path_buf.to_string_lossy().to_string();
        let path_exists = if published_path.is_empty() {
            false
        } else {
            Path::new(&published_path).exists()
        };
        ensure!(
            path_exists,
            "sui-base: could not find published data for package [{}]. Did you do '{} publish'?",
            package_name,
            workdir_name
        );

        path_buf.push(file_name);
        path_buf.set_extension(extension);

        Ok(path_buf.to_string_lossy().to_string())
    }

    fn get_pathname_state(
        &self,
        root: &mut SuiBaseRoot,
        state_name: &str,
    ) -> Result<String, anyhow::Error> {
        // Build pathname and do some error detections.
        ensure!(
            root.is_sui_base_installed(),
            "sui-base: not installed. Need to run ~/sui-base/install"
        );

        ensure!(
            !state_name.is_empty(),
            "sui-base: internal error invalid state_filename"
        );

        ensure!(
            self.workdir_name.is_some(),
            "sui-base: workdir name not set"
        );
        let workdir_name = self.workdir_name.as_ref().unwrap().to_string();

        ensure!(self.workdir_path.is_some(), "sui-base: workdir not set");
        let workdir_path = self.workdir_path.as_ref().unwrap().to_string();

        // Check if the state exists. If not, could be that the workdir need to be initialized.
        let mut path_buf = PathBuf::from(workdir_path);
        path_buf.push(".state");

        // Do an intermediate check for potential setup problem and provide an action to the user.
        let state_path = path_buf.to_string_lossy().to_string();
        let path_exists = if state_path.is_empty() {
            false
        } else {
            Path::new(&state_path).exists()
        };
        ensure!(
            path_exists,
            "sui-base: missing information from {}. You may need to initialize it first (e.g. do an update and/or start)",
            workdir_name
        );
        path_buf.push(state_name);
        Ok(path_buf.to_string_lossy().to_string())
    }

    fn get_url_from_state(
        &self,
        root: &mut SuiBaseRoot,
        url_field_name: &str,
    ) -> Result<String, anyhow::Error> {
        let pathname = self.get_pathname_state(root, "links")?;

        ensure!(
            self.workdir_name.is_some(),
            "sui-base: workdir name not set"
        );
        let workdir_name = self.workdir_name.as_ref().unwrap().to_string();

        // Load the link file, which is a JSON file.
        let file = File::open(pathname).with_context(|| {
            format!(
                "sui-base: workdir not fully initialized. Do '{0} start' or '{0} update'",
                workdir_name
            )
        })?;
        let reader = BufReader::new(file);
        let top: HashMap<String, Value> = serde_json::from_reader(reader)?;

        // Simply use the sui-base selected primary.
        let mut link_id: u64 = 0;
        if let Some(selection) = top.get("selection") {
            if let Some(primary_id) = selection.get("primary") {
                if let Some(primary_id_u64) = primary_id.as_u64() {
                    link_id = primary_id_u64; // id should be unique and always set.
                }
            }
        }

        if link_id == 0 {
            // This is unexpected, but do not fail. Just pick the first link in the config.
            //
            // Could be a transient problem, so do not prevent a RPC selection to be done.
            //
            // The user should be warn in some alternative ways (e.g. sui-base
            // health monitoring process).
            if let Some(links) = top.get("links") {
                if let Some(links_array) = links.as_array() {
                    if links_array.is_empty() {
                        bail!("sui-base: missing first link definition. Check sui-base.yaml links section.");
                    }
                    if let Some(first_link) = links_array.first() {
                        if let Some(rpc_v) = first_link.get(url_field_name) {
                            if let Some(rpc_str) = rpc_v.as_str() {
                                return Ok(rpc_str.to_string());
                            }
                        }
                    }
                }
            }
            bail!("sui-base: missing {} link field. May be a problem with the sui-base.yaml link section (1).", url_field_name );
        }

        // Get the information for that link.
        if let Some(links) = top.get("links") {
            if let Some(links_array) = links.as_array() {
                if links_array.is_empty() {
                    bail!("sui-base: missing at least one link definition. Check sui-base.yaml links section.");
                }
                for link in links_array {
                    if let Some(link_id_v) = link.get("id") {
                        if let Some(link_id_u64) = link_id_v.as_u64() {
                            if link_id_u64 == link_id {
                                if let Some(rpc_v) = link.get(url_field_name) {
                                    if let Some(rpc_str) = rpc_v.as_str() {
                                        return Ok(rpc_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        bail!("sui-base: missing {} link field. May be a problem with the sui-base.yaml link section (2).", url_field_name );
    }
}
