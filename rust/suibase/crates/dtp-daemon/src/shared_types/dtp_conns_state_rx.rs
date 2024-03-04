use common::basic_types::AutoSizeVecMapVec;

#[derive(Debug)]
// One per DTP connection.
pub struct DTPConnStateRxData {
    pub is_open: bool,
}

impl DTPConnStateRxData {
    pub fn new() -> Self {
        Self { is_open: false }
    }
}

impl std::default::Default for DTPConnStateRxData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GlobalsDTPConnsStateRxST {
    pub conns: AutoSizeVecMapVec<DTPConnStateRxData>,
}

impl GlobalsDTPConnsStateRxST {
    pub fn new() -> Self {
        Self {
            conns: AutoSizeVecMapVec::new(),
        }
    }
}

impl Default for GlobalsDTPConnsStateRxST {
    fn default() -> Self {
        Self::new()
    }
}
