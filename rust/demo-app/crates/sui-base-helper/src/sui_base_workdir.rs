use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::BufReader;

use serde_json::Value;

use sui_sdk::types::base_types::ObjectID;

use crate::sui_base_root::SuiBaseRoot;

use anyhow::{ensure, Context};

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
            "sui-base: not installed. Need to run ./install"
        );

        ensure!(
            !workdir_name.is_empty(),
            "sui-base: Invalid workdir name (empty string)"
        );

        // Check that the workdir do exists.
        let mut path_buf = PathBuf::from(root.workdirs_path());
        path_buf.push(workdir_name);
        let workdir_path = path_buf.to_string_lossy().to_string();

        let path_exists = if workdir_path.is_empty() {
            false
        } else {
            Path::new(&workdir_path).exists()
        };

        ensure!(path_exists, "sui-base: path [{}] not found", workdir_path);

        self.workdir_name = Some(workdir_name.to_string());
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

        // Simple parsing for a generated file expected to be: ["<hex string>"]

        // Trim new line.
        if in_str.ends_with('\n') {
            in_str.pop();
            if in_str.ends_with('\r') {
                in_str.pop();
            }
        }

        // Parse and remove the expected brackets and quotes.
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
        // TODO Implement this better with sui-base.yaml and/or ENV variables.
        //      See https://github.com/sui-base/sui-base/issues/6
        ensure!(
            root.is_sui_base_installed(),
            "sui-base: not installed. Need to run ./install"
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
            self.get_pathname_published_file(root, names[0], "publish-output", "json")?;

        // Load the publish-output.json file.
        let file = File::open(pathname)?;
        let reader = BufReader::new(file);
        let top: HashMap<String, Value> = serde_json::from_reader(reader)?;

        // TODO Consider caching optimization...

        // TODO Actually validate the object_type... but revisit this after release 0.28
        // because they are changing how TransactionEffects work. Might need to query
        // every object to check the type.

        // Path into the JSON file:
        //
        //  (1) "top" is Map "certificate", "effects"...
        //  (2) "effects" is Map "status", "executedEpoch", ..., "created, ...
        //  (3) "created" is Array of Map { "owner", "reference" }
        // This is the array that we want to iterate.
        // For each element we want to extract the "objectId" in the "reference" Map.
        //
        // Finally, we care only for Shared object (because they are hard to keep track of).
        let mut objects = vec![];

        // Works, looks terrible...
        if let Some(effects) = top.get("effects") {
            if let Some(created) = effects.get("created") {
                if let Some(created_array) = created.as_array() {
                    // Iterate the created objects.
                    for object_created in created_array {
                        if let Some(reference) = object_created.get("reference") {
                            if let Some(owner) = object_created.get("owner") {
                                if owner.is_string() {
                                    // That means it is an "Immutable". Ignore it, it is the package.
                                    continue;
                                }
                                if let Some(objectid_v) = reference.get("objectId") {
                                    if let Some(objectid_str) = objectid_v.as_str() {
                                        objects.push(ObjectID::from_hex_literal(objectid_str)?);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(objects)
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
            "sui-base: not installed. Need to run ./install"
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
}
