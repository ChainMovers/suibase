use common::basic_types::AutoSizeVecMapVec;

#[derive(Debug)]
// One per DTP connection.
pub struct DTPConnStateTxData {
    pub is_open: bool,
}

impl DTPConnStateTxData {
    pub fn new() -> Self {
        Self { is_open: false }
    }
}

impl std::default::Default for DTPConnStateTxData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GlobalsDTPConnsStateTxST {
    pub conns: AutoSizeVecMapVec<DTPConnStateTxData>,
}

impl GlobalsDTPConnsStateTxST {
    pub fn new() -> Self {
        Self {
            conns: AutoSizeVecMapVec::new(),
        }
    }
}

impl Default for GlobalsDTPConnsStateTxST {
    fn default() -> Self {
        Self::new()
    }
}
