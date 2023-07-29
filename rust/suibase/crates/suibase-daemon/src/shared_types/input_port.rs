use crate::basic_types::*;
use crate::shared_types::TargetServer;

use super::ServerStats;

use std::hash::Hasher;
use twox_hash::XxHash32;

#[derive(Debug)]
pub struct InputPort {
    idx: Option<ManagedVecUSize>,

    // The name of the workdir (e.g. localnet). Set once at contruction.
    workdir_name: String,

    // The workdir idx (from AdminController context). Set once at construction.
    workdir_idx: WorkdirIdx,

    // TCP/UDP port number. Set once at construction.
    port_number: u16,

    // Request that processing on this port be abandon.
    //
    // This is a irreversible request.
    //
    // This port configuration cannot be "re-activated" (the AdminController
    // must create another PortStates instance to re-use the same TCP/UDP port).
    deactivate_request: bool,

    // Indicate if a proxy_server thread is started or not for this port.
    proxy_server_running: bool,

    // Configuration. Can be change at runtime by the AdminController.
    pub target_servers: ManagedVec<TargetServer>,

    // Periodically updated by the NetworkMonitor.
    pub all_servers_stats: ServerStats,

    // The "TargetServer" selection vectors are updated periodically by
    // the NetworkMonitor. They help the handler to very quicly pick
    // a set of TargetServer to try.
    //
    // Design:
    //  All TargetServers in same selection[n] vector are considered of same
    //  quality (even if their health score is slightly different).
    //
    //  The choice of one or another server in a selection[n] vector is related
    //  to random load distribution.
    pub selection_vectors: Vec<Vec<TargetServerIdx>>,

    // All remaining TargetServerIdx that are not in selection_vectors because
    // not in OK state (could be fine rigt now, but not yet known). These are the
    // fallback attempts on initialization or hard recovery (least worst first).
    pub selection_worst: Vec<TargetServerIdx>,
}

impl InputPort {
    pub fn new(workdir_idx: WorkdirIdx, workdir_name: String, proxy_port_number: u16) -> Self {
        Self {
            idx: None,
            workdir_name,
            workdir_idx,
            port_number: proxy_port_number,
            deactivate_request: false,
            proxy_server_running: false,
            target_servers: ManagedVec::new(),
            all_servers_stats: ServerStats::new("all".to_string()),
            selection_vectors: Vec::new(),
            selection_worst: Vec::new(),
        }
    }

    pub fn add_target_server(&mut self, rpc: String, alias: String) {
        self.target_servers.push(TargetServer::new(rpc, alias));
    }

    pub fn workdir_idx(&self) -> WorkdirIdx {
        self.workdir_idx
    }

    pub fn workdir_name(&self) -> &str {
        &self.workdir_name
    }

    pub fn port_number(&self) -> u16 {
        self.port_number
    }

    pub fn deactivate(&mut self) {
        self.deactivate_request = true;
    }

    pub fn is_deactivated(&self) -> bool {
        self.deactivate_request
    }

    pub fn report_proxy_server_starting(&mut self) {
        self.proxy_server_running = true;
    }

    pub fn report_proxy_server_not_running(&mut self) {
        self.proxy_server_running = false;
    }

    fn find_best_target_server(&self) -> Option<(TargetServerIdx, String)> {
        let mut best_score: f64 = f64::MIN;
        let mut best_uri: String = String::new();
        let mut best_idx = None;

        // TODO integrate the user priority in this logic when there is no health_score yet!

        for (i, target_server) in self.target_servers.iter() {
            let score = target_server.health_score();
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
                best_uri = target_server.uri();
            }
        }

        Some((best_idx?, best_uri))
    }

    pub fn get_best_target_servers(
        &self,
        target_servers: &mut Vec<(TargetServerIdx, String)>,
        handler_start: &EpochTimestamp,
    ) {
        // Just leave target_servers untouch if there is any problem.

        if self.selection_vectors.is_empty() {
            // NetworkMonitor has not get a chance to run sufficiently yet, but the user
            // traffic is already coming in... so default to a simpler best server selection
            // that may rely more on the config user priority.
            if let Some((best_idx, best_uri)) = self.find_best_target_server() {
                target_servers.push((best_idx, best_uri));
            }
        } else {
            // Select up to 'RETRY_COUNT' healthy (when available).
            //
            // Get the first 'RETRY_COUNT' TargetServerIdx stored in self.selection_vectors[x][y] by incrementing x first then y.
            //
            // This allows to group TargetServer for load balancing and distribute evenly over a selection_vector[x].
            const RETRY_COUNT: usize = 3;
            let mut count = 0;
            let mut vector_idx: usize = 0;

            if self.selection_vectors.len() > 1 {
                // Load balance from the first selection_vectors (just pick a random starting point).
                let vector = &self.selection_vectors[vector_idx];
                vector_idx += 1;

                // Calculate how many more need to be pushed in target_servers.
                // Iterate the vector by starting at a random index and
                // wrapping around as needed.
                // Very weak "random" which is good enough. We just want to be
                // fast here. Proper distribution is compensated at a higher level
                // by the NetworkMonitor.
                let mut hasher = XxHash32::with_seed(0);
                hasher.write_u32(handler_start.elapsed().subsec_nanos());
                let rng = hasher.finish() as usize;
                for i in 0..vector.len() {
                    let idx = vector[(i + rng) % vector.len()];
                    if let Some(uri) = self.uri(idx) {
                        target_servers.push((idx, uri));
                        count += 1;
                        if count == RETRY_COUNT {
                            return; // Done
                        }
                    }
                }
            }

            // Select sequentially from this point on.
            for vector in &self.selection_vectors[vector_idx..] {
                for &idx in vector {
                    if let Some(uri) = self.uri(idx) {
                        target_servers.push((idx, uri));
                        count += 1;
                        if count == RETRY_COUNT {
                            return; // Done
                        }
                    }
                }
            }

            // If we are here, it means there was not enough healthy TargetServer so fallback
            // to choose among the worst selections (least known worst first).
            // Note: This can happen on initialization or hard recovery.
            for &idx in &self.selection_worst {
                if let Some(uri) = self.uri(idx) {
                    target_servers.push((idx, uri));
                    count += 1;
                    if count == RETRY_COUNT {
                        return; // Done
                    }
                }
            }
        }
    }

    pub fn uri(&self, server_idx: TargetServerIdx) -> Option<String> {
        self.target_servers.get(server_idx).map(|ts| ts.uri())
    }
}

impl ManagedElement for InputPort {
    fn idx(&self) -> Option<ManagedVecUSize> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecUSize>) {
        self.idx = index;
    }
}
