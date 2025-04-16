use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use common::basic_types::SafeUuid;

fn is_empty_string(s: &str) -> bool {
    s.is_empty()
}

#[serde_as]
#[derive(Clone, Default, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    // Header fields
    // =============
    //    - method:
    //        A string echoing the method of the request.
    //
    //    - key:
    //        A string echoing one of the "key" parameter of the request (e.g. the workdir requested).
    //        This field is optional and its interpretation depends on the method.
    //
    //    - data_uuid:
    //        A sortable hex 64 bytes (UUID v7). Increments with every data modification.
    //
    //    - method_uuid:
    //        A hex 64 bytes (UUID v4) that changes every time a new generated data_uuid is unexpectedly
    //        lower than the previous one for this method (e.g. system time went backward) or the PID of
    //        the process changes. Complements data_uuid for added reliability on various edge cases.
    //
    #[serde(skip_serializing_if = "is_empty_string")]
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semver: Option<String>, // Semantic versioning of the backend API.
}

// A trait for comparing two objects for equivalence, excluding the header fields (if any).
pub trait VersionedEq {
    fn versioned_eq(&self, other: &Self) -> bool;
}

impl Header {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.to_string(),
            method_uuid: None,
            data_uuid: None,
            key: None,
            semver: None,
        }
    }

    pub fn set_from_uuids(&mut self, uuids: &SafeUuid) {
        self.method_uuid = Some(uuids.get_method_uuid());
        self.data_uuid = Some(uuids.get_data_uuid());
    }
}

// Class to conveniently add UUID versioning to any data structure.
//
// That versioning can be used to initialize the method_uuid and data_uuid fields of a Header

// TODO Implement PartialEq and PartialOrd to use only the uuid field for comparison.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Versioned<T: VersionedEq> {
    uuid: SafeUuid,
    data: T,
}

impl<T: Clone + PartialEq + VersionedEq> Versioned<T> {
    pub fn new(data: T) -> Self {
        Self {
            uuid: SafeUuid::new(),
            data,
        }
    }

    // if data is different, then increment version, else no-op.
    pub fn set(&mut self, new_data: &T) -> SafeUuid {
        if !self.data.versioned_eq(new_data) {
            self.data = new_data.clone();
            self.uuid.increment();
        }
        self.uuid.clone()
    }

    pub fn take_if_not_equal(&mut self, new_data: T) -> bool {
        if !self.data.versioned_eq(&new_data) {
            self.data = new_data;
            self.uuid.increment();
            true
        } else {
            false
        }
    }

    // When owner did get_mut_data and made modifications to the data
    // directly, it must call this method to increment the version.
    pub fn inc_uuid(&mut self) {
        self.uuid.increment();
    }

    // readonly access
    pub fn get_data(&self) -> &T {
        &self.data
    }

    pub fn get_uuid(&self) -> &SafeUuid {
        &self.uuid
    }

    // write access
    pub fn get_mut_data(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn get_mut_uuid(&mut self) -> &mut SafeUuid {
        &mut self.uuid
    }

    // Write version into a Header structure.
    pub fn write_uuids_into_header_param(&self, header: &mut Header) {
        header.method_uuid = Some(self.uuid.get_method_uuid());
        header.data_uuid = Some(self.uuid.get_data_uuid());
    }
}
