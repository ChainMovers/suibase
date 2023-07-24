use crate::basic_types::*;

use crate::shared_types::ServerStats;

#[derive(Debug)]
pub struct TargetServer {
    managed_idx: Option<ManagedVecUSize>,
    pub stats: ServerStats,
    uri: String,
}

impl TargetServer {
    pub fn new(uri: String, alias: String) -> Self {
        Self {
            managed_idx: None,
            stats: ServerStats::new(alias),
            uri,
        }
    }

    pub fn health_score(&self) -> f64 {
        self.stats.health_score()
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
