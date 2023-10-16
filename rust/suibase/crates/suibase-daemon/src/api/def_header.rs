use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::shared_types::UuidST;

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
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

// Class to conveniently add UUID versioning to any data structure.
//
// That versioning can be used to initialize the method_uuid and data_uuid fields of a Header

// TODO Implement PartialEq and PartialOrd to use only the uuid field for comparison.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Versioned<T> {
    uuid: UuidST,
    data: T,
}

impl<T: Clone + PartialEq> Versioned<T> {
    pub fn new(data: T) -> Self {
        Self {
            uuid: UuidST::new(),
            data,
        }
    }

    // if data is different, then increment version, else no-op.
    pub fn set(&mut self, new_data: &T) -> UuidST {
        if new_data != &self.data {
            self.data = new_data.clone();
            self.uuid.increment();
        }
        self.uuid.clone()
    }

    // readonly access
    pub fn get_data(&self) -> &T {
        &self.data
    }

    pub fn get_uuid(&self) -> &UuidST {
        &self.uuid
    }

    // write access
    pub fn get_mut_data(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn get_mut_uuid(&mut self) -> &mut UuidST {
        &mut self.uuid
    }

    // Write version into a Header structure.
    pub fn init_header_uuids(&self, header: &mut Header) {
        let (method_uuid, data_uuid) = self.uuid.get();
        header.method_uuid = Some(method_uuid.to_string());
        header.data_uuid = Some(data_uuid.to_string());
    }
}
