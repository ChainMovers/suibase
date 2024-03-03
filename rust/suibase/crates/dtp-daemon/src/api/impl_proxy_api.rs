use tokio::sync::Mutex;

use axum::async_trait;

use jsonrpsee::core::RpcResult;

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE,
};
use crate::shared_types::{GlobalsProxyMT, ServerStats, UuidST};
use common::basic_types::TargetServerIdx;

use super::{InfoResponse, PingResponse, ProxyApiServer, VersionedEq};
use super::{LinkStats, LinksResponse, LinksSummary, RpcInputError};

use super::def_header::Versioned;

#[derive(Clone, PartialEq)]
struct GetLinksInput {
    pub target_servers_stats: Option<Vec<(TargetServerIdx, ServerStats)>>,
    pub all_servers_stats: Option<ServerStats>,
    pub selection_vectors: Option<Vec<Vec<u8>>>,
    pub input_port_found: bool,
    pub proxy_enabled: bool,
    pub user_request_start: bool,
}

impl GetLinksInput {
    pub fn new() -> Self {
        Self {
            target_servers_stats: None,
            all_servers_stats: None,
            selection_vectors: None,
            input_port_found: false,
            proxy_enabled: false,
            user_request_start: false,
        }
    }
}

impl VersionedEq for GetLinksInput {
    fn versioned_eq(&self, other: &Self) -> bool {
        self == other
    }
}

pub struct ProxyApiImpl {
    pub globals: GlobalsProxyMT,
    pub admctrl_tx: AdminControllerTx,
    prev_get_links_input: Mutex<Versioned<GetLinksInput>>,
}

impl ProxyApiImpl {
    pub fn new(globals: GlobalsProxyMT, admctrl_tx: AdminControllerTx) -> Self {
        let prev_get_links_input = Mutex::new(Versioned::new(GetLinksInput::new()));
        Self {
            globals,
            admctrl_tx,
            prev_get_links_input,
        }
    }

    fn fmt_f64_api(input: f64) -> String {
        // This function is used to format f64 metrics for the API.
        // Use empty string for min/max, NaN and infinite values.
        if input.is_finite() && input != f64::MAX && input != f64::MIN {
            format!("{:.2}", input)
        } else {
            "".to_string()
        }
    }

    fn fmt_str_ms(input: &str) -> String {
        // Transform input assuming it is a representing milliseconds
        // to be displayed within a 7 characters wide field.
        //
        // Output goes from "   0.00" to "9999.99".
        //
        // When input is >=9999.99, the output is ">10secs"
        // When input is <0.01, the output is   "  <0.01"
        // Empty, negative or bad input becomes  "      -"
        //
        let value = input.parse::<f64>().unwrap_or(-1.0);
        Self::fmt_f64_ms(value)
    }

    fn fmt_f64_ms(input: f64) -> String {
        // See fmt_str_ms for more info.
        if input.is_sign_negative() || !input.is_normal() {
            "      -".to_string()
        } else if input >= 9999.99f64 {
            ">10secs".to_string()
        } else if input < 0.01f64 {
            "  <0.01".to_string()
        } else {
            format!("{:7.2}", input)
        }
    }

    fn fmt_str_pct(input: &str) -> String {
        // Convert the input representing a positive percentage for display
        // within a field of fixed width of 5 characters.
        //
        // Expected input range is "0" or "0.0" to "100.0"
        //
        // Empty or invalid input is formatted as "    -"
        //
        // Any value above "100" is ignored.
        // Any non-numeric value is ignored.
        // Only one decimal is displayed (rounding applies).
        //
        // Examples:
        //      "0" is interpreted as x == 0 and formatted as      "  0.0"
        //    "0.0" is interpreted as 0 < x < 0.1 and formatted as " <0.1"
        //    "100" is interpreted as x == 100 and formatted as    "100.0"
        // "105.28" is interpreted as x == 100 and formatted as    "100.0"
        //   "0.19" is rounded to 0.2 and formatted as             "  0.2"
        let value = input
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>()
            .parse::<f64>()
            .unwrap_or(-1.0);

        Self::fmt_f64_pct(value)
    }

    fn fmt_f64_pct(input: f64) -> String {
        // See fmt_str_pct for more info.
        if input.is_sign_negative() || input.is_infinite() || input.is_nan() {
            return "    -".to_string();
        }

        if input == 0.0 {
            "  0.0".to_string()
        } else if input < 0.1 {
            " <0.1".to_string()
        } else if input >= 100.001 {
            "100.0".to_string()
        } else {
            format!("{:5.1}", input)
        }
    }

    fn fmt_str_score(input: &str) -> String {
        // Similar to fmt_str_pct, except:
        //   - 0.0 is shown as empty field (spaces).
        //   - accept negative values
        //   - have a +/- prefix.
        //   - Any invalid parsing will show as empty field.
        //
        // Always 6 characters wide.
        let value = input.parse::<f64>().unwrap_or(0.0);
        Self::fmt_f64_score(value)
    }

    fn fmt_f64_score(input: f64) -> String {
        // See fmt_str_score for more info.
        if input.is_infinite() || input.is_nan() {
            return "    -".to_string();
        }

        if input == 0.0 {
            "      ".to_string()
        } else if input.is_sign_positive() {
            if input < 0.1 {
                "  <0.1".to_string()
            } else if input >= 100.001 {
                "+100.0".to_string()
            } else {
                format!("{:+6.1}", input)
            }
        } else if input > -0.1 {
            "  -0.1".to_string()
        } else if input <= -100.0 {
            "-100.0".to_string()
        } else {
            format!("{:+6.1}", input)
        }
    }
    /*
        fn fmt_u64(input: u64) -> String {
            // Fix field width of 9 characters.
            //
            // if input >99999999 then return ">99999999"
            // if input is zero then return   "        0"
            // if u64:MAX then return         "        -"
            if input > 99999999 {
                ">99999999".to_string()
            } else if input == 0 {
                "        0".to_string()
            } else if input == u64::MAX {
                "        -".to_string()
            } else {
                format!("{:9}", input)
            }
        }

        fn fmt_u32(input: u32) -> String {
            // Same logic as fmt_u64.
            if input == u32::MAX {
                Self::fmt_u64(u64::MAX)
            } else {
                Self::fmt_u64(input as u64)
            }
        }
    */
}

#[async_trait]
impl ProxyApiServer for ProxyApiImpl {
    async fn get_links(
        &self,
        workdir: String,
        summary: Option<bool>,
        links: Option<bool>,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<LinksResponse> {
        let mut resp = LinksResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "getLinks".to_string();
        resp.header.key = Some(workdir.clone());

        // "Unwrap" all the options to booleans.
        //
        // Summary/links is the enabling of group of metrics.
        //
        // data/display/debug allow variations of how the output
        // is produced (and they may be combined).
        //

        // summary/links default to true when not specified.
        //
        // data/display/debug default to false when not specified
        // with the exception of data defaulting to true when
        // the other (display and debug) are false.
        //
        let summary = summary.unwrap_or(true);
        let links = links.unwrap_or(true);
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let data = data.unwrap_or(!(debug || display));

        let mut debug_out = String::new();

        // Variables initialized during the read lock.
        let mut inputs = GetLinksInput::new();
        let mut inputs_version: Option<UuidST> = None;

        {
            // Get read lock access to the globals and just quickly copy what is needed.
            // Most parsing and processing is done outside the lock.
            let globals_read_guard = self.globals.read().await;
            let globals = &*globals_read_guard;

            if let Some(input_port) = globals.find_input_port_by_name(&workdir) {
                inputs.input_port_found = true;
                inputs.proxy_enabled = input_port.is_proxy_enabled();
                inputs.user_request_start = input_port.is_user_request_start();

                inputs.all_servers_stats = Some(input_port.all_servers_stats.clone());

                let target_servers = &input_port.target_servers;

                inputs.target_servers_stats = Some(
                    target_servers
                        .iter()
                        .map(|(idx, target_server)| (idx, target_server.stats.clone()))
                        .collect(),
                );
                inputs.selection_vectors = Some(input_port.selection_vectors.clone());
            }

            // If debug, then extensively add more info to the output.
            // (take a potential performance hit here).
            if debug {
                debug_out.push_str(&format!("{:?}", globals));
            }

            // If data, then handle potential UUID increment.
            if data {
                // To avoid race condition, prev_get_links_input is lock and modified only here.
                // Outside the lock, use 'inputs' and 'inputs_version' within this thread.
                let prev_input = &mut *self.prev_get_links_input.lock().await;
                inputs_version = Some(prev_input.set(&inputs));
            }
        } // Release the read lock.

        // Map the target_servers_stats into the API LinkStats.
        let mut healthy_server_count: usize = 0;
        let mut neutral_health_count: usize = 0;
        let mut link_stats: Vec<LinkStats> = Vec::new();
        let mut load_distribution_depth = 0;
        if let Some(target_servers_stats) = inputs.target_servers_stats {
            let mut total_request: u64 = 0;
            let mut link_n_request: Vec<u64> = Vec::with_capacity(target_servers_stats.len());
            // Prepare LinkStats, which is the "metrics" portion of the API.
            //
            // The "display/debug" portion is built from the "metrics" portion.
            //
            // The design seems a bit inefficient (extra string conversion), but the
            // intention is to give more opportunity to catch bugs by using (earlier
            // than typical) the least visible (but crucial) metrics portion.

            // Use a vector of indices to drive the display order.
            // Also find which selections were assigned for load distribution (if any).
            let mut indices: Vec<usize> = Vec::new();
            if let Some(selection_vectors) = inputs.selection_vectors {
                if !selection_vectors.is_empty() {
                    load_distribution_depth = selection_vectors[0].len();
                }

                // Subtle transform. The selection_vectors managed idx are not the same as the "collect"
                // indices.
                let unmap_vec: Vec<u8> = selection_vectors.iter().flatten().copied().collect();
                for unmap_idx in unmap_vec {
                    // Find unmap_idx in target_servers_stats (first element of tuple) and
                    // remember the position of that element in target_servers_stats.
                    let idx = target_servers_stats
                        .iter()
                        .position(|(i, _)| *i == unmap_idx);
                    if let Some(idx) = idx {
                        indices.push(idx);
                    } else {
                        // That would be a bad bug in the selection logic... report it to dev.
                        log::error!("unmap_idx {} not found in target_servers_stats", unmap_idx);
                    }
                }
            } else {
                indices = Vec::with_capacity(target_servers_stats.len())
            };

            if indices.len() < target_servers_stats.len() {
                // Find the missing elements in indices.
                let mut missing_indices: Vec<usize> = (0..target_servers_stats.len()).collect();
                missing_indices.retain(|&i| !indices.contains(&i));
                // Sort the missing elements by alias.
                missing_indices.sort_by_key(|&i| target_servers_stats[i].1.alias());
                // Append to the final indices to be displayed.
                indices.extend(missing_indices);
            }

            for i in indices {
                let server_stats = &target_servers_stats[i].1;
                let mut link_stat = LinkStats::new(server_stats.alias());

                let mut n_request = 0u64;
                let mut n_success = 0u64;
                server_stats.get_accum_stats(&mut n_request, &mut n_success);
                total_request += n_request;
                if n_request != 0 {
                    let success_pct = (n_success as f64 * 100.0f64) / (n_request as f64);
                    link_stat.success_pct = Self::fmt_f64_api(success_pct);
                };

                let health_score = server_stats.health_score();
                if health_score.is_normal() && health_score.is_sign_positive() {
                    healthy_server_count += 1;
                }
                link_stat.health_pct = Self::fmt_f64_api(health_score);

                link_stat.resp_time = Self::fmt_f64_api(server_stats.avg_latency_ms());
                link_stat.error_info = server_stats.error_info();

                link_stat.status = if health_score == 0.0 {
                    // The server has not yet "determine" its initial health state.
                    neutral_health_count += 1;
                    String::new()
                } else if server_stats.is_healthy() {
                    "OK".to_string()
                } else {
                    "DOWN".to_string()
                };

                // Push always together for 1:1 index matching.
                link_stats.push(link_stat);
                link_n_request.push(n_request);
            }

            // Calculate the load_pct by iterating each link_stats.
            if total_request != 0 {
                for (i, link_stat) in link_stats.iter_mut().enumerate() {
                    let load_pct = (link_n_request[i] as f64 * 100.0f64) / (total_request as f64);
                    link_stat.load_pct = Self::fmt_f64_api(load_pct);
                }
            }
        }
        let link_stats = link_stats; // Make immutable.

        // Map the all_servers_stats into the API LinksSummary.
        let mut summary_stats = LinksSummary::new();

        if let Some(all_servers_stats) = inputs.all_servers_stats {
            summary_stats.success_on_first_attempt = all_servers_stats.success_on_first_attempt();
            summary_stats.success_on_retry = all_servers_stats.success_on_retry();
            all_servers_stats.get_classified_failure(
                &mut summary_stats.fail_network_down,
                &mut summary_stats.fail_bad_request,
                &mut summary_stats.fail_others,
            );
        }

        if !inputs.input_port_found {
            return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into());
        }

        // Identify the multi-link RPC status.
        let load_balance_str = if load_distribution_depth > 1 {
            ", load-balanced".to_string()
        } else {
            String::new()
        };

        let server_count = link_stats.len();
        (resp.status, resp.info) = if !inputs.proxy_enabled {
            ("DOWN".to_string(), "proxy not enabled".to_string())
        } else if !inputs.user_request_start {
            ("DOWN".to_string(), format!("{} not started", workdir))
        } else if server_count == 0 {
            ("DOWN".to_string(), "no links in suibase.yaml".to_string())
        } else if neutral_health_count == link_stats.len() {
            ("DOWN".to_string(), "initializing".to_string())
        } else if healthy_server_count == 0 {
            ("DOWN".to_string(), "no servers available".to_string())
        } else if healthy_server_count * 100 / server_count > 50 {
            let resp_info = if workdir == "localnet" {
                load_balance_str
            } else {
                format!("protected{}", load_balance_str)
            };
            ("OK".to_string(), resp_info)
        } else {
            (
                "OK".to_string(),
                format!(">50% degraded{}", load_balance_str),
            )
        };

        let mut display_out = String::new();

        if display {
            // User requested human-friendly display.
            if summary {
                let resp_info = if resp.info.is_empty() {
                    String::new()
                } else {
                    format!(" ( {} )", resp.info)
                };
                display_out.push_str(&format!(
                    "multi-link RPC: {}{}\n\n\
                    Cumulative Request Stats\n\
  -------------------------\n\
  Success first attempt {:>9}\n\
  Success after retry   {:>9}\n\
  Failure bad request   {:>9}\n\
  Failure others        {:>9}\n\n",
                    resp.status,
                    resp_info,
                    summary_stats.success_on_first_attempt,
                    summary_stats.success_on_retry,
                    summary_stats.fail_bad_request,
                    summary_stats.fail_others,
                ));
            }

            if links {
                display_out.push_str(
                    "alias                Status  Health%   Load%   RespT ms  Success%\n--------------------------------------------------------------------\n"
                );
                let mut load_distributed = load_distribution_depth;
                for link_stat in link_stats.iter() {
                    let load_dist_marker = if load_distributed > 0 {
                        load_distributed -= 1;
                        "*"
                    } else {
                        ""
                    };
                    display_out.push_str(&format!(
                        "{:<21}{:^6}{:1}{:>7}{:>8}{:>11}{:>10}  {}\n",
                        format!("{:.20}", link_stat.alias),
                        link_stat.status,
                        load_dist_marker,
                        Self::fmt_str_score(&link_stat.health_pct),
                        Self::fmt_str_pct(&link_stat.load_pct),
                        Self::fmt_str_ms(&link_stat.resp_time),
                        Self::fmt_str_pct(&link_stat.success_pct),
                        link_stat.error_info,
                    ));
                }
            }
            resp.display = Some(display_out);
        }

        if debug {
            // Communicate with AdminController to append its own debug state.
            let mut msg = AdminControllerMsg::new();
            msg.event_id = crate::admin_controller::EVENT_DEBUG_PRINT;
            let (tx, rx) = tokio::sync::oneshot::channel();
            msg.resp_channel = Some(tx);
            if (self.admctrl_tx.send(msg).await).is_ok() {
                match rx.await {
                    Ok(resp_str) => {
                        debug_out.push_str(&format!("\nAdminController: {}", resp_str));
                    }
                    Err(e) => {
                        debug_out.push_str(&format!("\nAdminController: {:?}", e));
                    }
                }
            }

            resp.debug = Some(debug_out);
        }

        if data {
            // User requested the raw stats.
            if summary {
                resp.summary = Some(summary_stats);
            }
            if links {
                resp.links = Some(link_stats);
            }

            if let Some(version) = inputs_version {
                resp.header.method_uuid = Some(version.get_method_uuid());
                resp.header.data_uuid = Some(version.get_data_uuid());
            }
        }

        Ok(resp)
    }

    async fn fs_change(&self, path: String) -> RpcResult<InfoResponse> {
        let mut resp = InfoResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "fsChange".to_string();

        // Inform the AdminController that something changed...
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_NOTIF_CONFIG_FILE_CHANGE;
        msg.data_string = Some(path);

        // TODO: Implement response to handle errors... but is it really needed here?
        let _ = self.admctrl_tx.send(msg).await;

        resp.info = "Success".to_string();
        Ok(resp)
    }

    async fn ping(
        &self,
        workdir: String,
        host_addr: String,
        bytes: Option<String>,
    ) -> RpcResult<PingResponse> {
        let mut resp = PingResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "ping".to_string();

        //
        // There is a unique index in a managed vector for each remote Host addr.
        //
        // The following global Managed vector exists:
        //   - One HostController for each HostIdx
        //     Maintain by the API side for its vector of ServiceController.
        //     Each ServiceController has a linked list of ConnsController.
        //     Each ConsController have a linked list of RequestPending.
        //
        //   - On the receive side, the IPipe must quickly map its data
        //     to its PipeController, which forward eventually to the
        //     proper ConnsController (RequestPending).
        //

        //
        // **** API Thread - Request Sequence ****
        //
        //   Find the HostSlaIdx.
        //
        //   HostSlaIdx is a U16 index unique for each (service_idx,host_addr,sla_idx) key tuple. See managed_vec_map_vec.rs.
        //
        //   As needed, the tx thread has to open the connection:
        //     Read lock on ConnsController[HostSlaIdx]
        //       If not open, then send open request to WebSocketWorker.
        //     Read unlock
        //     if open request sent, block wait until confirmed open (use a oneshot channel) or timeout.
        //
        //   Prepare for the TX:
        //   Write lock the TXController[HostSlaIdx]
        //     Run the TX state machine (as needed use WebSocketTXWorker for Move calls).
        //     Find the proper IPipe and sequence number.
        //   Write unlock
        //
        //   Prepare the RX side to expect a response:
        //   Write lock the RXController[HostSlaIdx]
        //     Add PendingRequest to it.
        //   Write unlock
        //
        //   Send the data once using WebSocketTXWorker (with IPipe, SeqNumber info).
        //
        //   Block wait on the oneshot channel for the response, failure or timeout destroy
        //
        // **** WebSocketTXWorker ****
        //   - On open connection request, do
        //       On open needed:
        //         Call into Sui network to open the connection (until success or timeout).
        //         Do RPC subscriptions (until success or timeout).
        //         Create TXController[HostSlaIdx] and RXController[HostSlaIdx].
        //         Write lock ConnsController[HostSlaIdx]
        //           Mark the connection as open.
        //         Write Unlock
        //     Send success/failure to caller with oneshot channel. Success if already open.
        //
        //   - On Exec Move Call (a oneshot chanel is provided)
        //       Send the requested operation on the websocket.
        //       On success:
        //         Write Lock RXControllerMoveCalls[HostSlaIdx]
        //           Add PendingRequest and move ownership of oneshot channel.
        //         Write Unlock

        //       On failure:
        //         Send failure to caller with oneshot channel.
        //
        // **** WebSocketRxWorker ****
        //  On peer data receive:
        //    Use the subscription ID to find HostSlaIdx (slow lookup).
        //    (Optimization: Response could include the HostSlaIdx for quick tentative lookup)
        //
        //    Write lock RXController[HostSlaIdx]
        //      Run RX State machine.
        //      Drop if invalid or no pending request.
        //      If valid, take ownership of the oneshot channel and delete PendingRequest.
        //    Write unlock.
        //    If own the oneshot channel, then send the response with the data.
        //
        //  On subscription-level event:
        //    Use the subscription ID to find HostSlaIdx (slow lookup).
        //    Run subscription state machine (does its own TX as needed)
        //
        //  On Move Call response:
        //    Use the returned HostSlaIdx for fast lookup.
        //    Write lock RXControllerMoveCalls[HostSlaIdx]
        //      Drop if invalid or no pending request.
        //      If valid, take ownership of the oneshot channel and delete PendingRequest.
        //    Write unlock.
        //    If own the oneshot channel, then send the response with the data.

        // Get the ConnsController for the Ping service.
        // (There is only one ConnsController for Ping
        //  service per HostController).

        // Send the ping request through the proper IPipe.
        // (for now use always the the first IPipe).

        Ok(resp)
    }
}
