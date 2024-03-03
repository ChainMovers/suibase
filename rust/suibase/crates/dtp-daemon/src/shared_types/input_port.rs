use crate::basic_types::*;
use crate::shared_types::Link;
use crate::shared_types::TargetServer;

use super::{ServerStats, WorkdirProxyConfig};

use std::hash::Hasher;
use twox_hash::XxHash32;

#[derive(Debug)]
pub struct InputPort {
    idx: Option<ManagedVecU8>,

    // The name of the workdir (e.g. localnet). Set once at construction.
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

    // Active Configuration.
    user_request_start: bool, // true when user_request == "start"
    proxy_enabled: bool,

    // Maintained by the AdminController such that the runtime idx remain the
    // same for a given alias ("forever", even when deleted from file config).
    pub target_servers: ManagedVec<TargetServer>,

    // Periodically updated by the NetworkMonitor.
    pub all_servers_stats: ServerStats,

    // The "TargetServer" selection vectors are updated periodically by
    // the NetworkMonitor. They help the handler to very quickly pick
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
    // not in OK state (could be fine right now, but not yet known). These are the
    // fallback attempts on initialization or hard recovery (least worst first).
    pub selection_worst: Vec<TargetServerIdx>,
}

impl InputPort {
    pub fn new(
        workdir_idx: WorkdirIdx,
        workdir_name: String,
        workdir_config: &WorkdirProxyConfig,
    ) -> Self {
        Self {
            idx: None,
            workdir_name,
            workdir_idx,
            port_number: workdir_config.proxy_port_number(),
            deactivate_request: false,
            proxy_server_running: false,
            user_request_start: workdir_config.is_user_request_start(),
            proxy_enabled: workdir_config.is_proxy_enabled(),
            target_servers: ManagedVec::new(),
            all_servers_stats: ServerStats::new("all".to_string()),
            selection_vectors: Vec::new(),
            selection_worst: Vec::new(),
        }
    }

    pub fn add_target_server(&mut self, config: &Link) {
        // Note: caller must make sure the alias does not exist already.
        self.target_servers.push(TargetServer::new(config.clone()));
    }

    pub fn upsert_target_server(&mut self, config: &Link) -> bool {
        // return true on any change.
        let mut at_least_one_change = false;

        // Linear search by alias among existing target servers.
        for (_, target_server) in self.target_servers.iter_mut() {
            if target_server.alias() == config.alias {
                // Handle modifications.
                if let Some(rpc) = config.rpc.as_ref() {
                    if &target_server.rpc() != rpc {
                        log::info!(
                            "{} modify server {} rpc from {} to {}",
                            self.workdir_name,
                            config.alias,
                            target_server.rpc(),
                            rpc
                        );
                        target_server.set_rpc(rpc.clone());
                        target_server.stats_clear();
                        at_least_one_change = true;
                    }

                    // Handle all other config changes in same way
                    // (without clearing the stats).
                    if target_server.get_config() != config {
                        log::info!(
                            "{} modify server {} params from {:?} to {:?}",
                            self.workdir_name,
                            config.alias,
                            target_server.get_config(),
                            config
                        );
                        target_server.set_config(config.clone());
                        at_least_one_change = true;
                    }
                }

                return at_least_one_change;
            }
        }
        // Does not exists... add it.
        log::info!("{} adding server {}", self.workdir_name, config.alias);
        self.add_target_server(config);
        true
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

    pub fn is_user_request_start(&self) -> bool {
        self.user_request_start
    }

    pub fn is_proxy_enabled(&self) -> bool {
        self.proxy_enabled
    }

    pub fn set_user_request_start(&mut self, value: bool) {
        self.user_request_start = value;
    }

    pub fn set_proxy_enabled(&mut self, value: bool) {
        self.proxy_enabled = value;
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
                best_uri = target_server.rpc();
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
        self.target_servers.get(server_idx).map(|ts| ts.rpc())
    }

    pub fn update_selection_vectors(&mut self) {
        let target_servers = &mut self.target_servers;

        self.selection_vectors.clear();
        self.selection_worst.clear();

        // Build a vector of idx() of the elements of target_servers.
        // At same time, find one currently OK with the best latency_avg().
        // Isolate immediately all down target servers in selection_worst.
        let mut ok_idx_vec: Vec<TargetServerIdx> = Vec::new();
        let mut best_latency_avg: f64 = f64::MAX;
        let mut best_latency_avg_idx: Option<TargetServerIdx> = None;
        for (_, target_server) in target_servers.iter() {
            if let Some(idx) = target_server.idx() {
                if target_server.stats.is_healthy() {
                    if best_latency_avg_idx.is_none()
                        || target_server.stats.avg_latency_ms() < best_latency_avg
                    {
                        best_latency_avg = target_server.stats.avg_latency_ms();
                        best_latency_avg_idx = Some(idx);
                    }
                    ok_idx_vec.push(idx);
                } else {
                    self.selection_worst.push(idx);
                }
            }
        }

        // If there is a best_latency_avg_idx, then this is the first element
        // in the first input_port.selection_vectors[0] to be created...
        // ... then join to it all the ok_idx_vec elements that are no more than
        // twice its latency avg (when below 250ms). Otherwise no more than 25%.
        //
        // This is the *best* bunch of target servers to be used for load balancing.
        //
        // All other ok_idx_vec elements are put in the second vector.
        if let Some(best_latency_avg_idx) = best_latency_avg_idx {
            self.selection_vectors
                .push(Vec::with_capacity(ok_idx_vec.len()));
            self.selection_vectors
                .push(Vec::with_capacity(ok_idx_vec.len()));

            self.selection_vectors[0].push(best_latency_avg_idx);

            let mut best_latency_avg = best_latency_avg;
            if best_latency_avg < 250.0 {
                best_latency_avg *= 2.0;
            } else {
                best_latency_avg *= 1.25;
            }
            for idx in ok_idx_vec.iter() {
                if best_latency_avg_idx == *idx {
                    continue;
                }
                if let Some(target_server) = target_servers.get(*idx) {
                    if target_server.stats.avg_latency_ms() <= best_latency_avg {
                        self.selection_vectors[0].push(*idx);
                    } else {
                        self.selection_vectors[1].push(*idx);
                    }
                }
            }
        } else {
            // This is for when there is not a single best_latency_avg_idx
            // (happens only briefly on process initialization).
            // TODO implement using user configuration priority instead to
            //      make two bins.
            self.selection_vectors.push(ok_idx_vec);
        }

        // Sort every selection_vectors by ascending latency.
        for vector in self.selection_vectors.iter_mut() {
            vector.sort_by(|a, b| {
                let a_server = target_servers.get(*a).unwrap();
                let b_server = target_servers.get(*b).unwrap();
                a_server
                    .stats
                    .avg_latency_ms()
                    .partial_cmp(&b_server.stats.avg_latency_ms())
                    .unwrap()
            });
        }

        if !self.selection_worst.is_empty() {
            // Sort input_port.selection_worst by increasing health_score()
            // and alias.
            self.selection_worst.sort_by(|a, b| {
                let a_server = target_servers.get(*a).unwrap();
                let b_server = target_servers.get(*b).unwrap();
                let a_score = a_server.health_score();
                let b_score = b_server.health_score();
                if a_score == b_score {
                    a_server.stats.alias().cmp(&b_server.stats.alias())
                } else {
                    a_score.partial_cmp(&b_score).unwrap()
                }
            });
        }
    }
}

impl ManagedElement for InputPort {
    fn idx(&self) -> Option<ManagedVecU8> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU8>) {
        self.idx = index;
    }
}
