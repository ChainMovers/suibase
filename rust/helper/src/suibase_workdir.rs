use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::BufReader;

use serde_json::Value;
use serde_yaml::Value as YamlValue;

use crate::error::Error;
use crate::suibase_root::SuibaseRoot;

// Validate + normalize a Sui object id / address hex string to the canonical
// "0x" + 64 lowercase hex form. Accepts an optional 0x/0X prefix and short
// forms (left zero-padded), matching the prior ObjectID::from_hex_literal /
// SuiAddress::from_str + to_string() behavior. Returns None if not valid hex
// or longer than 32 bytes. This keeps the helper free of any Sui-types
// dependency (see docs/dev/LOCALNET_WALRUS_PLAN.md — helper is a pure,
// always-buildable file reader).
fn normalize_sui_hex(input: &str) -> Option<String> {
    let s = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if s.is_empty() || s.len() > 64 {
        return None;
    }
    if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = String::with_capacity(66);
    out.push_str("0x");
    for _ in 0..(64 - s.len()) {
        out.push('0');
    }
    out.push_str(&s.to_ascii_lowercase());
    Some(out)
}

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

    pub(crate) fn package_id(
        &self,
        root: &mut SuibaseRoot,
        package_name: &str,
    ) -> Result<String, Error> {
        let pathname =
            self.get_pathname_published_file(root, package_name, "package-id", "json")?;
        let mut in_str = std::fs::read_to_string(&pathname).map_err(|io_error| {
            Error::PublishedDataAccessError {
                package_name: package_name.to_string(),
                path: pathname,
                io_error,
            }
        })?;

        in_str = in_str.trim().to_string();

        // Simple parsing for a generated file expected to be: ["<hex string>"]
        if !in_str.starts_with("[\"") || !in_str.ends_with("\"]") {
            return Err(Error::PackageIdJsonInvalidFormat);
        }
        let package_id_hex: &str = &in_str[2..in_str.len() - 2];

        // Parse the expected hex string.
        let package_id =
            normalize_sui_hex(package_id_hex).ok_or_else(|| Error::PackageIdInvalidHex {
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

    pub(crate) fn published_new_objects(
        &self,
        root: &mut SuibaseRoot,
        object_type: &str,
    ) -> Result<Vec<String>, Error> {
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
                            if let Some(objectid_field) = created_object.get("objectId") {
                                if let Some(objectid_str) = objectid_field.as_str() {
                                    objects.push(
                                        normalize_sui_hex(objectid_str).ok_or_else(|| {
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

    pub(crate) fn client_address(
        &self,
        root: &mut SuibaseRoot,
        address_name: &str,
    ) -> Result<String, Error> {
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
                        return normalize_sui_hex(address_str).ok_or_else(|| {
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
        path_buf.push("most-recent");

        let symlink = std::fs::read_link(&path_buf);
        if let Ok(symlink_target) = symlink {
            // Do resolve the symlink portion as part of the original path.
            let canonical_path = std::fs::canonicalize(&path_buf);
            if let Ok(resolved_path) = canonical_path {
                path_buf = resolved_path;
            } else {
                return Err(Error::PublishedDataAccessErrorInvalidSymlink {
                    package_name: package_name.to_string(),
                    path: path_buf.to_string_lossy().to_string(),
                    symlink_target: symlink_target.to_string_lossy().to_string(),
                });
            }
        } else {
            return Err(Error::PublishedDataAccessErrorSymlinkNotFound {
                package_name: package_name.to_string(),
                path: path_buf.to_string_lossy().to_string(),
            });
        }

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
                path: published_path,
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

        // Simply use the suibase selected primary.
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

    fn get_client_active_address(&self, root: &mut SuibaseRoot) -> Result<String, Error> {
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
        let sui_address = normalize_sui_hex(active_addr).ok_or_else(|| {
            Error::ConfigActiveAddressParseError {
                address: active_addr.to_string(),
            }
        })?;

        Ok(sui_address) // Success!
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_sui_hex;

    #[test]
    fn normalize_canonical_full_length() {
        let canon = "0x6c2547cbbc38025cf3adac45f63cb0a8d12ecf777cdc75a4971612bf97fdf6af";
        assert_eq!(normalize_sui_hex(canon).as_deref(), Some(canon));
    }

    #[test]
    fn normalize_lowercases_and_keeps_length_66() {
        let upper = "0x6C2547CBBC38025CF3ADAC45F63CB0A8D12ECF777CDC75A4971612BF97FDF6AF";
        let out = normalize_sui_hex(upper).unwrap();
        assert_eq!(out.len(), 66);
        assert!(out.starts_with("0x"));
        assert_eq!(out, upper.to_ascii_lowercase());
    }

    #[test]
    fn normalize_pads_short_forms_like_from_hex_literal() {
        // from_hex_literal("0x2") -> 0x000..02 (32-byte left-padded), to_string canonical.
        assert_eq!(
            normalize_sui_hex("0x2").as_deref(),
            Some("0x0000000000000000000000000000000000000000000000000000000000000002")
        );
        // No prefix is accepted too.
        assert_eq!(
            normalize_sui_hex("2").as_deref(),
            Some("0x0000000000000000000000000000000000000000000000000000000000000002")
        );
    }

    #[test]
    fn normalize_rejects_invalid() {
        assert_eq!(normalize_sui_hex(""), None); // empty
        assert_eq!(normalize_sui_hex("0x"), None); // empty after prefix
        assert_eq!(normalize_sui_hex("0xzz"), None); // non-hex
        // 65 hex chars (> 32 bytes) is too long.
        assert_eq!(normalize_sui_hex(&"a".repeat(65)), None);
    }
}
