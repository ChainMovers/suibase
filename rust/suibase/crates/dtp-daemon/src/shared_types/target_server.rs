use common::basic_types::*;

use crate::shared_types::Link;
use crate::shared_types::ServerStats;

#[derive(Debug)]
pub struct TargetServer {
    idx: Option<ManagedVecU8>,
    config: Link,
    pub stats: ServerStats,
}

impl TargetServer {
    pub fn new(config: Link) -> Self {
        // alias is the 'key' and can't be changed after construction.
        let alias = config.alias.clone();
        Self {
            idx: None,
            config,
            stats: ServerStats::new(alias),
        }
    }

    pub fn alias(&self) -> String {
        self.config.alias.clone()
    }

    pub fn health_score(&self) -> f64 {
        self.stats.health_score()
    }

    pub fn rpc(&self) -> String {
        self.config
            .rpc
            .as_ref()
            .map_or_else(String::new, |rpc| rpc.clone())
    }

    pub fn set_rpc(&mut self, rpc: String) {
        self.config.rpc = Some(rpc);
    }

    pub fn is_selectable(&self) -> bool {
        self.config.selectable
    }

    pub fn is_monitored(&self) -> bool {
        self.config.monitored
    }

    pub fn stats_clear(&mut self) {
        self.stats.clear();
    }

    pub fn get_config(&self) -> &Link {
        &self.config
    }

    pub fn set_config(&mut self, config: Link) {
        self.config = config
    }
}

impl ManagedElement for TargetServer {
    fn idx(&self) -> Option<ManagedVecU8> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU8>) {
        self.idx = index;
    }
}
