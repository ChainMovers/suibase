use std::collections::HashMap;

use crate::{basic_types::*, shared_types::InputPort};

use crate::shared_types::{
    GlobalsProxyMT, GlobalsProxyST, RequestFailedReason, SendFailedReason, ServerStats,
    TargetServer, REQUEST_FAILED_BAD_REQUEST_HTTP, SEND_FAILED_RESP_HTTP_STATUS,
    SEND_FAILED_UNSPECIFIED_STATUS,
};
use crate::workers::RequestWorker;

use bitflags::bitflags;

use anyhow::{anyhow, Result};
use hyper::http;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use tokio::time::{Duration, Instant};

pub const HEADER_SBSD_SERVER_IDX: &str = "X-SBSD-SERVER-IDX";
pub const HEADER_SBSD_SERVER_HC: &str = "X-SBSD-SERVER-HC";

pub struct NetmonMsg {
    // Internal messaging. Sent for every user request/response.
    // Purposely pack this in a few bytes for performance reason.
    event_id: NetmonEvent,
    flags: NetmonFlags,
    port_idx: u8,
    server_idx: u8,
    // Interpretation depends on the event_id.
    timestamp: EpochTimestamp,
    para32: [u32; 2],
    para8: [u8; 2],
    para16: [u16; 1],
}

impl NetmonMsg {
    pub fn new() -> Self {
        Self {
            event_id: 0,
            flags: NetmonFlags::empty(),
            port_idx: u8::MAX,
            server_idx: u8::MAX,
            timestamp: Instant::now(),
            para32: [0; 2],
            para8: [0; 2],
            para16: [0; 1],
        }
    }
    pub fn server_idx(&self) -> u8 {
        self.server_idx
    }

    pub fn para32(&self) -> &[u32; 2] {
        &self.para32
    }

    pub fn para16(&self) -> &[u16; 1] {
        &self.para16
    }

    pub fn para8(&self) -> &[u8; 2] {
        &self.para8
    }
}

// Events ID
pub type NetmonEvent = u8;
pub const EVENT_GLOBALS_AUDIT: u8 = 1; // Periodic read-only audit of the globals. May trig other events.
pub const EVENT_REPORT_REQ_FAILED: u8 = 2; // proxy_server reporting stats on a request dropped (not sent after retries).
pub const EVENT_REPORT_TGT_REQ_RESP_OK: u8 = 3; // proxy_server reporting stats on a successful request/response.
pub const EVENT_REPORT_TGT_REQ_RESP_ERR: u8 = 4; // proxy_server reporting stats on a response indicating an error.
pub const EVENT_REPORT_TGT_SEND_FAILED: u8 = 5; // proxy_server reporting stats on a failed send attempt.
pub const EVENT_DO_SERVER_HEALTH_CHECK: u8 = 6; // Start an async health check (a request/response test) for one server.

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NetmonFlags: u8 {
        const NEED_GLOBAL_WRITE_MUTEX = 0x01;
        const NEED_GLOBAL_READ_MUTEX = 0x02;
        const HEADER_SBSD_SERVER_IDX_SET = 0x04;
        const HEADER_SBSD_SERVER_HC_SET = 0x08;
    }
}

impl std::fmt::Debug for NetmonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut flags = String::new();
        if self.flags.intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX) {
            flags.push_str("NEED_GLOBAL_WRITE_MUTEX ");
        }
        if self.flags.intersects(NetmonFlags::NEED_GLOBAL_READ_MUTEX) {
            flags.push_str("NEED_GLOBAL_READ_MUTEX ");
        }
        write!(
            f,
            "NetmonMsg {{ event_id: {}, flags: {}, port_idx: {}, server_idx: {}, timestamp: {:?}, para32: {:?} }}",
            self.event_id, flags, self.port_idx, self.server_idx, self.timestamp, self.para32
        )
    }
}

pub type NetMonTx = tokio::sync::mpsc::Sender<NetmonMsg>;
pub type NetMonRx = tokio::sync::mpsc::Receiver<NetmonMsg>;

struct MonitorData {
    most_recent_latency_test_attempted: Option<EpochTimestamp>,
}

impl MonitorData {
    pub fn new() -> Self {
        Self {
            most_recent_latency_test_attempted: None,
        }
    }
}

pub struct NetworkMonitor {
    globals: GlobalsProxyMT,
    netmon_rx: NetMonRx,
    mon_map: HashMap<(InputPortIdx, TargetServerIdx), MonitorData>,
}

// This is how the ProxyHandler communicate with the NetworkMonitor.
// It creates a ProxyHandlerReport instance and call into it.
pub struct ProxyHandlerReport<'a> {
    tx_channel: &'a NetMonTx,
    flags: NetmonFlags,
    port_idx: InputPortIdx,
    handler_start: EpochTimestamp,
}

impl<'a> ProxyHandlerReport<'a> {
    pub fn new(
        tx_channel: &'a NetMonTx,
        port_idx: InputPortIdx,
        handler_start: EpochTimestamp,
    ) -> Self {
        Self {
            tx_channel,
            flags: NetmonFlags::empty(),
            port_idx,
            handler_start,
        }
    }

    pub fn mut_flags(&mut self) -> &mut NetmonFlags {
        &mut self.flags
    }

    pub async fn req_resp_ok(
        &mut self,
        server_idx: TargetServerIdx,
        req_initiation_time: EpochTimestamp,
        resp_received: EpochTimestamp,
        retry_count: u8,
    ) -> Result<()> {
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_REPORT_TGT_REQ_RESP_OK;
        self.flags.insert(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX);
        msg.flags = self.flags;
        msg.port_idx = self.port_idx;
        msg.server_idx = server_idx;
        msg.timestamp = req_initiation_time;
        msg.para32[0] = duration_to_micros(req_initiation_time - self.handler_start);
        msg.para32[1] = duration_to_micros(resp_received - req_initiation_time);
        msg.para8[0] = retry_count;

        // Send the message.
        self.tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    pub async fn req_resp_err(
        &mut self,
        server_idx: TargetServerIdx,
        req_initiation_time: EpochTimestamp,
        resp_received: EpochTimestamp,
        retry_count: u8,
        reason: RequestFailedReason,
    ) -> Result<()> {
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_REPORT_TGT_REQ_RESP_ERR;
        self.flags.insert(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX);
        msg.flags = self.flags;
        msg.port_idx = self.port_idx;
        msg.server_idx = server_idx;
        msg.timestamp = req_initiation_time;
        msg.para32[0] = duration_to_micros(req_initiation_time - self.handler_start);
        msg.para32[1] = duration_to_micros(resp_received - req_initiation_time);
        msg.para8[0] = retry_count;
        msg.para8[1] = reason;

        // Send the message.
        self.tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    pub async fn req_fail(&mut self, retry_count: u8, reason: RequestFailedReason) -> Result<()> {
        let error_time = EpochTimestamp::now();
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_REPORT_REQ_FAILED;
        self.flags.insert(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX);
        msg.flags = self.flags;
        msg.port_idx = self.port_idx;
        msg.server_idx = 0; // Not used.
        msg.timestamp = error_time;
        msg.para32[0] = duration_to_micros(error_time - self.handler_start);
        msg.para8[0] = retry_count;
        msg.para8[1] = reason;

        // Send the message.
        self.tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    pub async fn send_failed(
        &mut self,
        server_idx: TargetServerIdx,
        req_initiation_time: EpochTimestamp,
        reason: SendFailedReason,
        status: http::StatusCode,
    ) -> Result<()> {
        let error_time = EpochTimestamp::now();
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_REPORT_TGT_SEND_FAILED;
        self.flags.insert(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX);
        msg.flags = self.flags;
        msg.port_idx = self.port_idx;
        msg.server_idx = server_idx;
        msg.timestamp = req_initiation_time;
        msg.para32[0] = duration_to_micros(req_initiation_time - self.handler_start);
        msg.para32[1] = duration_to_micros(error_time - req_initiation_time);
        msg.para8[1] = reason;
        msg.para16[0] = status.as_u16();

        // Send the message.
        self.tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    // Return true if the cause of the error is
    // the server and the request is likely
    // to succeed with another server.
    //
    // Return false if these is clear indication
    // that the request is bad and will fail
    // with any server.
    pub async fn http_response_error(
        &mut self,
        server_idx: &u8,
        req_initiation_time: EpochTimestamp,
        _resp_received: EpochTimestamp,
        retry_count: u8,
        err: &reqwest::Error,
    ) -> bool {
        if let Some(status) = err.status() {
            // If the HTTP error is cause by a bad client request, then fail
            // the request (client's fault, the server did nothing wrong...)
            match status {
                http::StatusCode::BAD_REQUEST
                | http::StatusCode::METHOD_NOT_ALLOWED
                | http::StatusCode::UNSUPPORTED_MEDIA_TYPE => {
                    // These will not punish the server health score.
                    let _ = self
                        .req_fail(retry_count, REQUEST_FAILED_BAD_REQUEST_HTTP)
                        .await;
                    // Do not try another server.
                    return false;
                }
                _ => {}
            }
            // Assume it is a server health problem (punish the server health score).
            let _ = self
                .send_failed(
                    *server_idx,
                    req_initiation_time,
                    SEND_FAILED_RESP_HTTP_STATUS,
                    status,
                )
                .await;
        } else {
            // Unspecified error (punish the server health score).
            let _ = self
                .send_failed(
                    *server_idx,
                    req_initiation_time,
                    SEND_FAILED_UNSPECIFIED_STATUS,
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
                .await;
        }

        // Try another server.
        true
    }
}

impl NetworkMonitor {
    pub fn new(globals: GlobalsProxyMT, netmon_rx: NetMonRx, _netmon_tx: NetMonTx) -> Self {
        Self {
            globals,
            netmon_rx,
            mon_map: HashMap::new(),
        }
    }

    pub async fn send_event_globals_audit(tx_channel: &NetMonTx) -> Result<()> {
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_GLOBALS_AUDIT;
        msg.flags = NetmonFlags::NEED_GLOBAL_READ_MUTEX;
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    // Message that the NetworkManager sends to itself.
    //
    // A "ReadLock" section send this message to a "WriteLock" section.
    //
    async fn send_do_server_health_check(
        tx_channel: &NetMonTx,
        port_idx: InputPortIdx,
        server_idx: TargetServerIdx,
        port_number: u16,
    ) -> Result<()> {
        // TODO: Bug protection against health check flooding!?
        let mut msg = NetmonMsg::new();
        msg.event_id = EVENT_DO_SERVER_HEALTH_CHECK;
        msg.port_idx = port_idx;
        msg.server_idx = server_idx;
        msg.para16[0] = port_number;

        // Send the message.
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    async fn process_latency_report_attempt_request(
        mon_map: &mut HashMap<(u8, u8), MonitorData>,
        request_worker_tx: &NetMonTx,
        port_idx: InputPortIdx,
        server_idx: TargetServerIdx,
        port_number: u16,
        now: EpochTimestamp,
        force: bool,
    ) {
        let mon_data = mon_map
            .entry((port_idx, server_idx))
            .or_insert(MonitorData::new());

        let ts = &mon_data.most_recent_latency_test_attempted;
        if force || ts.is_none() || (now - ts.unwrap()) > Duration::from_secs(15) {
            // Let the request worker take care of this.
            let _ = NetworkMonitor::send_do_server_health_check(
                request_worker_tx,
                port_idx,
                server_idx,
                port_number,
            )
            .await;

            mon_data.most_recent_latency_test_attempted = Some(now);
        }
    }

    async fn process_read_only_globals(
        &mut self,
        msg: NetmonMsg,
        request_worker_tx: &NetMonTx,
    ) -> Option<NetmonMsg> {
        // Process messages that requires READ only access to the globals.
        //
        // All the NetmonMsg are process single threaded (by the netmon thread).
        //
        if !msg.flags.intersects(NetmonFlags::NEED_GLOBAL_READ_MUTEX) {
            // Do not consume the message.
            return Some(msg);
        }

        let now = EpochTimestamp::now();

        {
            let globals_read_guard = self.globals.read().await;
            let globals = &*globals_read_guard;
            let input_ports = &globals.input_ports;

            let mut cur_msg = msg;
            loop {
                match cur_msg.event_id {
                    EVENT_GLOBALS_AUDIT => {
                        for (_, input_port) in input_ports.iter() {
                            if let Some(port_idx) = input_port.idx() {
                                // Iterate every target_servers.
                                for (_, target_server) in input_port.target_servers.iter() {
                                    if let Some(server_idx) = target_server.idx() {
                                        Self::process_latency_report_attempt_request(
                                            &mut self.mon_map,
                                            request_worker_tx,
                                            port_idx,
                                            server_idx,
                                            input_port.port_number(),
                                            now,
                                            false,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        log::debug!(
                            "process_read_only_globals unexpected event id {}",
                            cur_msg.event_id
                        );
                        return None; // Consume the bad message.
                    }
                }

                // Check if more messages are available.
                match self.netmon_rx.try_recv() {
                    Ok(next_msg) => {
                        cur_msg = next_msg;
                    }
                    Err(_e) => {
                        // No more messages.
                        return None;
                    }
                }

                if !cur_msg
                    .flags
                    .intersects(NetmonFlags::NEED_GLOBAL_READ_MUTEX)
                {
                    // Does not requires a global read mutex.
                    // Do not consume that message here.
                    return Some(cur_msg);
                }
            }
        }
    }

    fn get_mut_all_servers_stats<'a>(
        input_ports: &'a mut ManagedVec<InputPort>,
        msg: &NetmonMsg,
    ) -> Option<&'a mut ServerStats> {
        if let Some(input_port) = input_ports.get_mut(msg.port_idx) {
            return Some(&mut input_port.all_servers_stats);
        }

        log::debug!("input port {} not found", msg.port_idx);
        None
    }

    fn get_mut_target_server<'a>(
        input_ports: &'a mut ManagedVec<InputPort>,
        msg: &NetmonMsg,
    ) -> Option<&'a mut TargetServer> {
        if let Some(input_port) = input_ports.get_mut(msg.port_idx) {
            if let Some(target_server) = input_port.target_servers.get_mut(msg.server_idx) {
                /*log::debug!(
                    "found target_server {} with index {} found",
                    target_server.stats.alias(),
                    msg.server_idx
                );*/
                return Some(target_server);
            }
        }

        log::debug!(
            "input port {} target server {} not found",
            msg.port_idx,
            msg.server_idx
        );
        None
    }

    fn update_selection_vectors(input_ports: &mut ManagedVec<InputPort>, msg: &NetmonMsg) {
        if let Some(input_port) = input_ports.get_mut(msg.port_idx) {
            input_port.update_selection_vectors();
        }
    }

    async fn process_mut_globals(&mut self, msg: NetmonMsg) -> Option<NetmonMsg> {
        // Process messages that requires WRITE access to the globals.
        //
        // All the NetmonMsg are process single threaded (by the netmon thread).
        //
        if !msg.flags.intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX) {
            // Do not consume the message.
            return Some(msg);
        }

        {
            let mut globals_write_guard = self.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_ports = &mut globals.input_ports;

            let mut cur_msg = msg;
            loop {
                match cur_msg.event_id {
                    EVENT_REPORT_TGT_REQ_RESP_OK => {
                        // Update the stats. Consume the message.
                        if cur_msg
                            .flags
                            .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                        {
                            // This is for the "controlled" latency test.
                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server
                                    .stats
                                    .handle_latency_report(cur_msg.timestamp, cur_msg.para32[1]);

                                // Always update the selection_vectors on a good latency_report. This is
                                // the periodic "audit" opportunity to refresh things up.
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        } else {
                            // This is for the user traffic.
                            if let Some(stats) = crate::NetworkMonitor::get_mut_all_servers_stats(
                                input_ports,
                                &cur_msg,
                            ) {
                                stats.handle_resp_ok(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                );
                            }

                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server.stats.handle_resp_ok(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                );
                            }
                        }
                    }
                    EVENT_REPORT_TGT_REQ_RESP_ERR => {
                        // Update the stats.
                        if cur_msg
                            .flags
                            .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                        {
                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                let was_healthy = target_server.stats.is_healthy();

                                // This is for the "controlled" latency test.
                                // We do not want that failure to mix with the user
                                // traffic stats so call report_req_failed_internal
                                // instead.
                                target_server.stats.handle_req_failed_internal(
                                    cur_msg.timestamp,
                                    cur_msg.para8[1],
                                );

                                // A bad latency report on a healthy target_server could affect
                                // the selection of the target server.
                                if was_healthy {
                                    Self::update_selection_vectors(input_ports, &cur_msg);
                                }
                            }
                        } else {
                            // An error in the response for the user traffic.
                            if let Some(stats) = crate::NetworkMonitor::get_mut_all_servers_stats(
                                input_ports,
                                &cur_msg,
                            ) {
                                stats.handle_resp_err(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                    cur_msg.para8[1],
                                );
                            }

                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server.stats.handle_resp_err(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                    cur_msg.para8[1],
                                );
                                // User traffic should not select that target again.
                                // So always refresh the selection_vectors on every user
                                // traffic error.
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        }
                    }
                    EVENT_REPORT_TGT_SEND_FAILED => {
                        // An error just sending a request.
                        if let Some(target_server) =
                            NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                        {
                            let was_healthy = target_server.stats.is_healthy();

                            target_server.stats.handle_send_failed(
                                cur_msg.timestamp,
                                cur_msg.para8[1],
                                cur_msg.para16[0],
                            );

                            let update_selection_vectors = if cur_msg
                                .flags
                                .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                            {
                                was_healthy
                            } else {
                                true
                            };

                            if update_selection_vectors {
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        }
                    }
                    EVENT_REPORT_REQ_FAILED => {
                        // Update the stats. Not related to a specific target server
                        // so update only the all_servers stats.
                        if let Some(stats) =
                            crate::NetworkMonitor::get_mut_all_servers_stats(input_ports, &cur_msg)
                        {
                            if cur_msg
                                .flags
                                .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                            {
                                stats.handle_req_failed_internal(
                                    cur_msg.timestamp,
                                    cur_msg.para8[1],
                                );
                            } else {
                                stats.handle_req_failed(cur_msg.timestamp, cur_msg.para8[1]);
                            }
                        }
                    }
                    _ => {
                        log::error!(
                            "process_mut_globals unexpected event id {}",
                            cur_msg.event_id
                        );
                        // Do nothing. Consume the bad message.
                    }
                }

                // Check if more messages are available.
                match self.netmon_rx.try_recv() {
                    Ok(next_msg) => {
                        cur_msg = next_msg;
                    }
                    Err(_e) => {
                        // No more messages.
                        return None;
                    }
                }

                if !cur_msg
                    .flags
                    .intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX)
                {
                    // Does not requires a global mutex.
                    // Do not consume that message here.
                    return Some(cur_msg);
                }
            }
        }
    }

    async fn process_msg(
        &mut self,
        msg: NetmonMsg,
        request_worker_tx: &NetMonTx,
    ) -> Option<NetmonMsg> {
        // Process messages that do not require any global mutex.
        //
        // All the NetmonMsg are process single threaded (by the netmon thread).
        //
        if msg
            .flags
            .intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX | NetmonFlags::NEED_GLOBAL_READ_MUTEX)
        {
            // Do not consume the message.
            return Some(msg);
        }

        {
            let mut cur_msg = msg;
            loop {
                match cur_msg.event_id {
                    EVENT_DO_SERVER_HEALTH_CHECK => {
                        // Forward to request worker.
                        Self::process_latency_report_attempt_request(
                            &mut self.mon_map,
                            request_worker_tx,
                            cur_msg.port_idx,
                            cur_msg.server_idx,
                            cur_msg.para16[0],
                            EpochTimestamp::now(),
                            true,
                        )
                        .await;
                    }
                    _ => {
                        log::debug!("process_msg unexpected event id {}", cur_msg.event_id);
                        return None; // Consume the bad message.
                    }
                }
                // Check if more messages are available.
                match self.netmon_rx.try_recv() {
                    Ok(next_msg) => {
                        cur_msg = next_msg;
                    }
                    Err(_e) => {
                        // No more messages.
                        return None;
                    }
                }

                if cur_msg.flags.intersects(
                    NetmonFlags::NEED_GLOBAL_WRITE_MUTEX | NetmonFlags::NEED_GLOBAL_READ_MUTEX,
                ) {
                    // Requires a global mutex.
                    // Do not consume that message here.
                    return Some(cur_msg);
                }
            }
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle, request_worker_tx: NetMonTx) {
        let mut cur_msg: Option<NetmonMsg> = Option::None;

        while !subsys.is_shutdown_requested() {
            if cur_msg.is_none() {
                // Wait for a message.
                cur_msg = self.netmon_rx.recv().await;
                if cur_msg.is_none() || subsys.is_shutdown_requested() {
                    // Channel closed or shutdown requested.
                    return;
                }
            }

            // Do processing of consecutive messages that requires READ only globals mutex (if any)
            cur_msg = self
                .process_read_only_globals(cur_msg.unwrap(), &request_worker_tx)
                .await;

            if cur_msg.is_none() {
                continue;
            }

            // Do processing of consecutive messages that requires WRITE globals mutex (if any)
            cur_msg = self.process_mut_globals(cur_msg.unwrap()).await;

            if cur_msg.is_none() {
                continue;
            }

            // Do processing of consecutive messages that do not requires globals.
            cur_msg = self.process_msg(cur_msg.unwrap(), &request_worker_tx).await;
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // Start another thread to initiate requests toward target servers (e.g. health check)
        let (request_worker_tx, request_worker_rx) = tokio::sync::mpsc::channel(1000);
        let request_worker = RequestWorker::new(request_worker_rx);
        subsys.start("request-worker", move |a| request_worker.run(a));

        // The loop to handle all incoming messages.
        match self
            .event_loop(&subsys, request_worker_tx)
            .cancel_on_shutdown(&subsys)
            .await
        {
            Ok(()) => {
                log::info!("shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}
