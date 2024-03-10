use common::basic_types::{ManagedElement16, ManagedVecMapVec, ManagedVecU16};
use dtp_sdk::{DTP,Host};

#[derive(Debug)]
// One per DTP connection.
pub struct DTPConnStateData {
    pub idx: Option<ManagedVecU16>,
    pub is_open: bool,
    pub dtp: Option<DTP>,
}

impl DTPConnStateData {
    pub fn new() -> Self {
        Self {
            idx: None,
            is_open: false,
            dtp: None,
        }
    }
}

impl std::default::Default for DTPConnStateData {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedElement16 for DTPConnStateData {
    fn idx(&self) -> Option<ManagedVecU16> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU16>) {
        self.idx = index;
    }
}

#[derive(Debug)]
pub struct GlobalsDTPConnsStateST {
    pub conns: ManagedVecMapVec<DTPConnStateData>,
}

impl GlobalsDTPConnsStateST {
    pub fn new() -> Self {
        Self {
            conns: ManagedVecMapVec::new(),
        }
    }
}

impl Default for GlobalsDTPConnsStateST {
    fn default() -> Self {
        Self::new()
    }
}
