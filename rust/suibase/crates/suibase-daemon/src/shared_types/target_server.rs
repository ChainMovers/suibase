use crate::basic_types::*;

use crate::shared_types::ServerStats;

pub struct TargetServer {
    managed_idx: Option<ManagedVecUSize>,
    pub stats: ServerStats,
    uri: String,
}

impl TargetServer {
    pub fn new(uri: String) -> Self {
        Self {
            managed_idx: None,
            stats: ServerStats::new(),
            uri,
        }
    }

    pub fn relative_health_score(&self) -> i8 {
        self.stats.relative_health_score()
    }

    pub fn uri(&self) -> String {
        self.uri.clone()
    }
}

impl ManagedElement for TargetServer {
    fn managed_idx(&self) -> Option<ManagedVecUSize> {
        self.managed_idx
    }

    fn set_managed_idx(&mut self, index: Option<ManagedVecUSize>) {
        self.managed_idx = index;
    }
}
