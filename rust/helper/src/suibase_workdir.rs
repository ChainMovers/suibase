use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

use serde_json::Value;
use serde_yaml::Value as YamlValue;

use sui_types::base_types::{ObjectID, SuiAddress};

use crate::error::Error;
use crate::suibase_root::SuibaseRoot;

pub(crate) struct SuibaseWorkdir {
    workdir_name: Option<String>,
    workdir_path: Option<String>,
}

impl SuibaseWorkdir {
    pub fn new() -> SuibaseWorkdir {
        SuibaseWorkdir {
            workdir_name: None,
            workdir_path: None,
        }
    }

    pub(crate) fn init_from_existing(
        &mut self,
        root: &mut SuibaseRoot,
        workdir_name: &str,
    ) -> Result<(), Error> {
        if !root.is_installed() {
            return Err(Error::NotInstalled);
        }

        if workdir_name.is_empty() {
            return Err(Error::WorkdirNameEmpty);
        }

        // Check that the workdir do exists.
        let mut path_buf = PathBuf::from(root.workdirs_path());
        path_buf.push(workdir_name);
        path_buf = std::fs::canonicalize(path_buf).map_err(|_| Error::WorkdirAccessError)?;

        let workdir_path = path_buf.to_string_lossy().to_string();
        let path_exists = if workdir_path.is_empty() {
            false
        } else {
            Path::new(&workdir_path).exists()
        };

        if !path_exists {
            return Err(Error::WorkdirNotExists);
        }

        // Get the actual workdir name from the .state/name
        //
        // It resolved the workdir name when "active", but also, it generally provides
        // a sanity check that the workdir was created and is read accessible by this app.
        path_buf.push(".state");
        path_buf.push("name");
        let mut in_str =
            std::fs::read_to_string(&path_buf).map_err(|_| Error::WorkdirAccessError)?;
        in_str = in_str.trim().to_string();
        if in_str.is_empty() {
            return Err(Error::WorkdirStateNameNotSet);
        }

        self.workdir_name = Some(in_str);
        self.workdir_path = Some(workdir_path);
        Ok(())
    }

    pub(crate) fn get_name(&self) -> Result<String, Error> {
        // That should be called only when the workdir was selected
        // and the name is set, but still check to avoid panic.
        if self.workdir_name.is_none() {
            return Err(Error::WorkdirNameNotSet);
        }

        // Safe to unwrap, because is_some() checked and then use
        // to_string to make a copy to bubble up.
        Ok(self.workdir_name.as_ref().unwrap().to_string())
    }

    pub(crate) fn package_object_id(
        &self,
        root: &mut SuibaseRoot,
        package_name: &str,
    ) -> Result<ObjectID, Error> {
        let pathname =
            self.get_pathname_published_file(root, package_name, "package-id", "json")?;

        // TODO: add info from inner I/O error.
        let mut in_str = std::fs::read_to_string(&pathname)
            .map_err(|_| Error::PublishedDataAccessError { path: pathname })?;

        in_str = in_str.trim().to_string();

        // Simple parsing for a generated file expected to be: ["<hex string>"]
        if !in_str.starts_with("[\"") || !in_str.ends_with("\"]") {
            return Err(Error::PackageIdJsonInvalidFormat);
        }
        let package_id_hex: &str = &in_str[2..in_str.len() - 2];

        // Parse the expected hex string.
        let package_id =
            ObjectID::from_hex_literal(package_id_hex).map_err(|_| Error::PackageIdInvalidHex {
                id: package_id_hex.to_string(),
            })?;
        Ok(package_id)
    }

    pub(crate) fn keystore_pathname(&self, root: &mut SuibaseRoot) -> Result<String, Error> {
        if !root.is_installed() {
            return Err(Error::NotInstalled);
        }

        if self.workdir_path.is_none() {
            return Err(Error::WorkdirPathNotSet);
        }
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

        if !keystore_file_exists {
            return Err(Error::SuibaseKeystoreNotExists {
                path: keystore_file,
            });
        }

        Ok(keystore_file)
    }

    pub(crate) fn published_new_object_ids(
        &self,
        root: &mut SuibaseRoot,
        object_type: &str,
    ) -> Result<Vec<ObjectID>, Error> {
        // Validate the parameter format.
        let mut names = vec![];
        for found in object_type.split("::") {
            let trim_str = found.trim();
            // A name can't be whitespaces.
            if trim_str.is_empty() {
                return Err(Error::ObjectTypeMissingField);
            }
            names.push(trim_str);
        }
        if names.len() != 3 {
            return Err(Error::ObjectTypeInvalidFormat);
        }

        let pathname: &str =
            &self.get_pathname_published_file(root, names[0], "created-objects", "json")?;

        // Load the created-objects.json file.
        let file = File::open(pathname).map_err(|_| Error::PublishedNewObjectAccessError {
            path: pathname.to_string(),
        })?;
        let reader = BufReader::new(file);
        let top: Value =
            serde_json::from_reader(reader).map_err(|_| Error::PublishedNewObjectReadError {
                path: pathname.to_string(),
            })?;

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
                                    objects.push(
                                        ObjectID::from_hex_literal(objectid_str).map_err(|_| {
                                            Error::PublishedNewObjectParseError {
                                                path: pathname.to_string(),
                                                id: objectid_str.to_string(),
                                            }
                                        })?,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(objects)
    }

    pub(crate) fn client_sui_address(
        &self,
        root: &mut SuibaseRoot,
        address_name: &str,
    ) -> Result<SuiAddress, Error> {
        // Validate the parameters.
        if address_name.is_empty() {
            return Err(Error::AddressNameEmpty);
        }

        if address_name == "active" {
            // Different logic equivalent to "sui client active-address"
            return self.get_client_active_address(root);
        }

        let pathname: &str = &self.get_pathname_state(root, "dns")?;

        // Load the dns file, which is a JSON file.
        let file = File::open(pathname).map_err(|_| Error::WorkdirStateDNSAccessFailed {
            path: pathname.to_string(),
        })?;

        let reader = BufReader::new(file);
        let top: HashMap<String, Value> =
            serde_json::from_reader(reader).map_err(|_| Error::WorkdirStateDNSReadError {
                path: pathname.to_string(),
            })?;

        if let Some(known) = top.get("known") {
            if let Some(known_item) = known.get(address_name) {
                if let Some(address_v) = known_item.get("address") {
                    if let Some(address_str) = address_v.as_str() {
                        return SuiAddress::from_str(address_str).map_err(|_| {
                            Error::WorkdirStateDNSParseError {
                                path: pathname.to_string(),
                                address: address_str.to_string(),
                            }
                        });
                    }
                }
            }
        }
        Err(Error::AddressNameNotFound {
            address_name: address_name.to_string(),
        })
    }

    pub(crate) fn rpc_url(&self, root: &mut SuibaseRoot) -> Result<String, Error> {
        self.get_url_from_state(root, "rpc")
    }

    pub(crate) fn ws_url(&self, root: &mut SuibaseRoot) -> Result<String, Error> {
        self.get_url_from_state(root, "ws")
    }
}

impl SuibaseWorkdir {
    //*************************************************/
    // This scope is for the private utility functions.
    //*************************************************/
    fn get_pathname_published_file(
        &self,
        root: &mut SuibaseRoot,
        package_name: &str,
        file_name: &str,
        extension: &str,
    ) -> Result<String, Error> {
        // Build pathname and do some error detections.

        if !root.is_installed() {
            return Err(Error::NotInstalled);
        }

        if package_name.is_empty() {
            return Err(Error::PackageNameEmpty);
        }

        if file_name.is_empty() {
            return Err(Error::FileNameEmpty);
        }

        if self.workdir_name.is_none() {
            return Err(Error::WorkdirNameNotSet);
        }
        let workdir_name = self.workdir_name.as_ref().unwrap().to_string();

        if self.workdir_path.is_none() {
            return Err(Error::WorkdirPathNotSet);
        }
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

        if !path_exists {
            return Err(Error::PublishedDataNotFound {
                package_name: package_name.to_string(),
                workdir: workdir_name,
            });
        }

        path_buf.push(file_name);
        path_buf.set_extension(extension);

        Ok(path_buf.to_string_lossy().to_string())
    }

    fn get_pathname_state(
        &self,
        root: &mut SuibaseRoot,
        state_name: &str,
    ) -> Result<String, Error> {
        // Build pathname and do some error detections.
        if !root.is_installed() {
            return Err(Error::NotInstalled);
        }

        if state_name.is_empty() {
            return Err(Error::StateNameEmpty);
        }

        if self.workdir_name.is_none() {
            return Err(Error::WorkdirNameNotSet);
        }
        let workdir_name = self.workdir_name.as_ref().unwrap().to_string();

        if self.workdir_path.is_none() {
            return Err(Error::WorkdirPathNotSet);
        }
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
        if !path_exists {
            return Err(Error::WorkdirInitializationIncomplete {
                workdir: workdir_name,
            });
        }

        path_buf.push(state_name);
        Ok(path_buf.to_string_lossy().to_string())
    }

    fn get_url_from_state(
        &self,
        root: &mut SuibaseRoot,
        url_field_name: &str,
    ) -> Result<String, Error> {
        let pathname: &str = &self.get_pathname_state(root, "links")?;

        if self.workdir_name.is_none() {
            return Err(Error::WorkdirNameNotSet);
        }
        let workdir_name: &str = &self.workdir_name.as_ref().unwrap().to_string();

        // Load the link file, which is a JSON file.
        let file = File::open(pathname).map_err(|_| Error::WorkdirInitializationIncomplete {
            workdir: workdir_name.to_string(),
        })?;
        let reader = BufReader::new(file);
        let top: HashMap<String, Value> =
            serde_json::from_reader(reader).map_err(|_| Error::WorkdirStateLinkReadError {
                path: pathname.to_string(),
            })?;

        // Simply use the suibaseselected primary.
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
            // The user should be warn in some alternative ways (e.g. suibase
            // health monitoring process).
            if let Some(links) = top.get("links") {
                if let Some(links_array) = links.as_array() {
                    if links_array.is_empty() {
                        return Err(Error::MissingLinkDefinition);
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
            return Err(Error::MissingLinkField {
                url_field: url_field_name.to_string(),
            });
        }

        // Get the information for that link.
        if let Some(links) = top.get("links") {
            if let Some(links_array) = links.as_array() {
                if links_array.is_empty() {
                    return Err(Error::MissingAtLeastOneLinkDefinition);
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

        Err(Error::MissingLinkField {
            url_field: url_field_name.to_string(),
        })
    }

    fn get_client_active_address(&self, root: &mut SuibaseRoot) -> Result<SuiAddress, Error> {
        // Directly access and parse the client.yaml.
        if !root.is_installed() {
            return Err(Error::NotInstalled);
        }

        if self.workdir_name.is_none() {
            return Err(Error::WorkdirNameNotSet);
        }
        let workdir_name: &str = &self.workdir_name.as_ref().unwrap().to_string();

        if self.workdir_path.is_none() {
            return Err(Error::WorkdirPathNotSet);
        }
        let workdir_path = self.workdir_path.as_ref().unwrap().to_string();

        // Check if the config directory is available (and resolve symlinks)
        let mut path_buf = PathBuf::from(workdir_path);
        path_buf.push("config");
        path_buf.push("client");
        path_buf.set_extension("yaml");
        path_buf = std::fs::canonicalize(path_buf).map_err(|_| Error::ConfigAccessError {
            workdir: workdir_name.to_string(),
        })?;
        let pathname = path_buf.to_string_lossy().to_string();

        // Try to open the file.
        let file = File::open(pathname).map_err(|_| Error::ConfigAccessError {
            workdir: workdir_name.to_string(),
        })?;
        let reader = BufReader::new(file);
        let data: YamlValue =
            serde_yaml::from_reader(reader).map_err(|_| Error::ConfigReadError {
                workdir: workdir_name.to_string(),
            })?;

        // Try to parse the "active_address" YAML field
        let active_addr: &str = &data["active_address"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(Error::ConfigActiveAddressParseError {
                address: "<missing>".to_string(),
            })?;
        let sui_address = SuiAddress::from_str(active_addr).map_err(|_| {
            Error::ConfigActiveAddressParseError {
                address: active_addr.to_string(),
            }
        })?;

        Ok(sui_address) // Success!
    }
}
