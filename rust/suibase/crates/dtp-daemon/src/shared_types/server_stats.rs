// Maintains stats/health of a server (IP:Port).

use hyper::http;

use crate::basic_types::*;

type UpScoreBonus = f64;
const NORMAL_SCORE_UP: UpScoreBonus = 1.15;
const WEAK_SCORE_UP: UpScoreBonus = 1.01;

const SLOW_LATENCY_LIMIT_MICROSECONDS: u32 = 4_000_000; // 4 seconds

// Request Failure Reasons
// !!! Append new reasons at the end and update REQUEST_FAILED_LAST_REASON
pub type RequestFailedReason = u8;
pub const REQUEST_FAILED_BODY_READ: u8 = 0;
pub const REQUEST_FAILED_NO_SERVER_RESPONDING: u8 = 1;
pub const REQUEST_FAILED_NO_SERVER_AVAILABLE: u8 = 2;
pub const REQUEST_FAILED_RESP_BYTES_RX: u8 = 3;
pub const REQUEST_FAILED_RESP_BUILDER: u8 = 4;
pub const REQUEST_FAILED_NETWORK_DOWN: u8 = 5; // Not implemented yet.
pub const REQUEST_FAILED_BAD_REQUEST_HTTP: u8 = 6; // Got HTTP Bad Request (400), Bad Method (405), etc.
pub const REQUEST_FAILED_BAD_REQUEST_JSON: u8 = 7; // Got a valid JSON-RPC response indicating an error.
pub const REQUEST_FAILED_CONFIG_DISABLED: u8 = 8;
pub const REQUEST_FAILED_NOT_STARTED: u8 = 9;

// !!! Update the following whenever you append a new reason above.
pub const REQUEST_FAILED_LAST_REASON: u8 = REQUEST_FAILED_NOT_STARTED;

// Do not touch this.
pub const REQUEST_FAILED_VEC_SIZE: usize = REQUEST_FAILED_LAST_REASON as usize + 1;

// Send Failure Reasons
// !!! Append new reasons at the end and update REQUEST_FAILED_LAST_REASON
pub type SendFailedReason = u8;
pub const SEND_FAILED_UNSPECIFIED_ERROR: u8 = 0;
pub const SEND_FAILED_RESP_HTTP_STATUS: u8 = 1;
pub const SEND_FAILED_UNSPECIFIED_STATUS: u8 = 2;

// !!! Update the following whenever you append a new reason above.
pub const SEND_FAILED_LAST_REASON: u8 = SEND_FAILED_UNSPECIFIED_STATUS;

// Do not touch this.
pub const SEND_FAILED_VEC_SIZE: usize = SEND_FAILED_LAST_REASON as usize + 1;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerStats {
    // Keep a copy of the server alias here because it is very
    // practical later while processing API calls.
    alias: String,

    // is_healthy reflects the latest known request/response result.
    // "Latest" means in "order of initiated requested order".
    //
    // If a success (or failure) is reported *after* but *initiated* before
    // "most_recent_initiated_timestamp" then it is ignored for "is_healthy".
    //
    is_healthy: bool,
    most_recent_initiated_timestamp: EpochTimestamp,

    // Window avg for successful response that are also intended to be used
    // for latency measurements.
    // Complete failure does not affect the latency measurements.
    //
    // Like for is_healthy, if a reported latency is process out-of-order (from when
    // they were initiated), then the oldest one is ignored.
    latency_report_avg: f64,
    latency_report_most_recent: Option<EpochTimestamp>,
    latency_report_count: u64,

    success_on_first_attempt: u64,
    success_on_retry: u64,
    retry_count: u64,
    // Theses are specific failure counts for request.
    //
    // There could be multiple send failure (retries) per
    // request so these are counted separately.
    //
    // unknown_reason are for when something out-of-range
    // is reported (that would be a bug).
    req_failure_reasons: [u64; REQUEST_FAILED_VEC_SIZE],
    req_unknown_reason: u64,
    send_failure_reasons: [u64; SEND_FAILED_VEC_SIZE],
    send_unknown_reason: u64,

    // These are internal request and do not count
    // as user traffic failure. Example is the
    // the health check request.
    req_failure_internal: u64,

    // Health management variables.
    //
    // Penalty and bonus for sequential good/bad reports.
    //
    // Value should be '0' for no-effect.
    up_score: f64,   // Value from 0 to 100
    down_score: f64, // Value from 0 to 100

    error_info: Option<String>, // Info on most recent failure.
}

impl ServerStats {
    pub fn new(alias: String) -> Self {
        let now = EpochTimestamp::now();
        Self {
            alias,

            is_healthy: false,
            most_recent_initiated_timestamp: now,

            latency_report_avg: f64::MAX,
            latency_report_most_recent: None,

            latency_report_count: 0,
            success_on_first_attempt: 0,
            success_on_retry: 0,
            retry_count: 0,

            req_failure_reasons: [0; REQUEST_FAILED_VEC_SIZE],
            req_unknown_reason: 0,

            send_failure_reasons: [0; SEND_FAILED_VEC_SIZE],
            send_unknown_reason: 0,

            req_failure_internal: 0,

            up_score: 0.0,
            down_score: 0.0,

            error_info: None,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.alias.clone());
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn error_info(&self) -> String {
        if self.error_info.is_none() {
            String::new()
        } else {
            self.error_info.as_ref().unwrap().clone()
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    pub fn avg_latency_ms(&self) -> f64 {
        self.latency_report_avg
    }

    pub fn success_on_first_attempt(&self) -> u64 {
        self.success_on_first_attempt
    }

    pub fn success_on_retry(&self) -> u64 {
        self.success_on_retry
    }

    fn get_accum_failure(&self) -> u64 {
        let mut total = 0;
        for i in 0..REQUEST_FAILED_VEC_SIZE {
            total += self.req_failure_reasons[i];
        }
        total += self.req_unknown_reason;
        total
    }

    // A few convenient function for the API processing the stats.
    pub fn get_accum_stats(&self, sum_request: &mut u64, sum_success: &mut u64) {
        *sum_success = self.success_on_first_attempt + self.success_on_retry;
        *sum_request = *sum_success + self.get_accum_failure();
    }

    pub fn get_classified_failure(
        &self,
        network_down: &mut u64,
        bad_request: &mut u64,
        other_failures: &mut u64,
    ) {
        // Sum all the request failures.
        let total = self.get_accum_failure();

        // Now isolate a few notable one for the caller.
        *network_down = self.req_failure_reasons[REQUEST_FAILED_NETWORK_DOWN as usize];
        *bad_request = self.req_failure_reasons[REQUEST_FAILED_BAD_REQUEST_HTTP as usize];
        *other_failures = total - (*network_down + *bad_request);
    }

    pub fn latency_report_most_recent(&self) -> Option<EpochTimestamp> {
        self.latency_report_most_recent
    }

    fn is_client_fault(reason: RequestFailedReason) -> bool {
        // Identify reason for which the failure can be
        // attributed to the client doing a bad request.
        matches!(reason, REQUEST_FAILED_BAD_REQUEST_HTTP)
    }

    pub fn handle_resp_ok(
        &mut self,
        initiation_time: EpochTimestamp,
        retry_count: u8,
        _prep_microsecs: u32,
        _latency_microsecs: u32,
    ) {
        self.inc_up_score(initiation_time, NORMAL_SCORE_UP);
        if retry_count == 0 {
            self.success_on_first_attempt += 1;
        } else {
            self.success_on_retry += 1;
        }
    }

    pub fn handle_resp_err(
        &mut self,
        initiation_time: EpochTimestamp,
        retry_count: u8,
        _prep_microsecs: u32,
        _latency_microsecs: u32,
        reason: RequestFailedReason,
    ) {
        // Do first like report_req_failed() and then handle some
        // additional information related to the response.
        self.handle_req_failed(initiation_time, reason);
        if retry_count != 0 {
            self.retry_count += retry_count as u64;
        }
    }

    pub fn handle_req_failed(
        &mut self,
        initiation_time: EpochTimestamp,
        reason: RequestFailedReason,
    ) {
        if !Self::is_client_fault(reason) {
            self.inc_down_score(initiation_time);
        }

        if reason >= self.req_failure_reasons.len() as u8 {
            log::debug!("internal error oob array access: {}", reason);
            self.req_unknown_reason += 1;
        } else {
            self.req_failure_reasons[reason as usize] += 1;
        }
    }

    pub fn handle_req_failed_internal(
        &mut self,
        initiation_time: EpochTimestamp,
        reason: RequestFailedReason,
    ) {
        // An example of internal request is the health check.
        //
        // A failure of it would end up here and transition
        // the server to unhealthy.
        if !Self::is_client_fault(reason) {
            self.inc_down_score(initiation_time);
        }

        self.req_failure_internal += 1;
    }

    pub fn handle_send_failed(
        &mut self,
        initiation_time: EpochTimestamp,
        reason: SendFailedReason,
        status: u16,
    ) {
        self.inc_down_score(initiation_time);
        if reason >= self.send_failure_reasons.len() as u8 {
            log::debug!("internal error oob array access: {}", reason);
            self.send_unknown_reason += 1;
        } else {
            self.send_failure_reasons[reason as usize] += 1;
            match reason {
                SEND_FAILED_UNSPECIFIED_ERROR => {
                    self.error_info = Some("Server Unreachable".to_string())
                }
                SEND_FAILED_RESP_HTTP_STATUS => {
                    let status_code = http::StatusCode::from_u16(status);
                    match status_code {
                        Ok(status_code) => {
                            if let Some(reason) = status_code.canonical_reason() {
                                self.error_info = Some(format!("({}){}", status, reason));
                            } else {
                                self.error_info = Some(format!("{}-HTTP-StatusCode", status));
                            }
                        }
                        Err(_) => {
                            self.error_info = Some(format!("{}-HTTP-StatusCode", status));
                        }
                    };
                }
                _ => self.error_info = Some("".to_string()),
            }
        }
    }

    pub fn handle_latency_report(
        &mut self,
        initiation_time: EpochTimestamp,
        latency_microsecs: u32,
    ) {
        log::debug!(
            "ServerStats::report_latency() for {} with latency_microsecs: {}",
            self.alias,
            latency_microsecs,
        );

        // Check for large value that would hint to a bug.
        let mut latency_microsecs = latency_microsecs;
        if latency_microsecs > MICROSECOND_LIMIT {
            latency_microsecs = MICROSECOND_LIMIT;
            log::error!("ServerStats::report_latency() clamped");
        }

        let bonus = if latency_microsecs >= SLOW_LATENCY_LIMIT_MICROSECONDS {
            WEAK_SCORE_UP
        } else {
            NORMAL_SCORE_UP
        };

        if self.latency_report_most_recent.is_none() {
            // One-time initialization
            self.latency_report_most_recent = Some(initiation_time);
            self.latency_report_avg = latency_microsecs as f64 / 1000.0; // to milliseconds.
            self.latency_report_count = 1;
            // Reflect that the server is healthy, but do not give too
            // much of a bonus if extremely slow (>4 secs).
            self.inc_up_score(initiation_time, bonus);
            return;
        }

        if initiation_time < self.latency_report_most_recent.unwrap() {
            // Unexpected out-of-order reception of this report. Avoid using
            // it for further effect.
            return;
        }

        // This is a valid latency report.
        self.latency_report_most_recent = Some(initiation_time);
        self.latency_report_count += 1;

        // Reflect that the server was healthy (at least at the moment the request was initiated).
        self.inc_up_score(initiation_time, bonus);

        // Use a 20 measurements exponential moving average to smooth out the latency.
        const ALPHA_AND_CONV: f64 = 0.00005; // 0.05 / 1000 (1000 is for microsecs to millisecs)
        const ONE_MINUS_ALPHA: f64 = 0.95; // 1.0 - 0.05

        self.latency_report_avg =
            self.latency_report_avg * ONE_MINUS_ALPHA + latency_microsecs as f64 * ALPHA_AND_CONV;
    }

    // A score from -100 to 100 about the health of this server.
    //
    // Any positive value are for a server UP (can be used for request).
    // Any negative value are for a server currently down (should not be used).
    //
    // The health score attempts to gauge how "good" or "bad" a server
    // is doing relative to each others.
    //
    // The longer a server is down, the more negative the score.
    //
    // The longer a server is up, the more positive the score.
    //
    pub fn health_score(&self) -> f64 {
        if self.down_score == 0.0 && self.up_score == 0.0 {
            // Still the initialization values, so be neutral.
            return 0.0;
        }

        if self.is_healthy {
            self.up_score
        } else {
            -self.down_score
        }
    }

    fn inc_up_score(&mut self, initiation_time: EpochTimestamp, bonus: UpScoreBonus) {
        // Note: There is no 'dec_up_score()'. See 'inc_down_score()' for how
        //       the up_score is slowly reduced.
        if initiation_time > self.most_recent_initiated_timestamp {
            // First update the boolean state which is either healthy or not.
            self.is_healthy = true;
            self.most_recent_initiated_timestamp = initiation_time;

            // Clear any potential previous error info.
            if self.error_info.is_some() {
                self.error_info = None;
            }

            // Every subsequent healthy report gives a bonus...
            if self.up_score < 1.0 {
                self.up_score = 1.0;
            } else if self.up_score < 100.0 {
                let new_value = self.up_score * bonus;
                self.up_score = new_value.min(100.0);
            }

            // Slowly reduce the "down_score" (for next time it is down).
            if self.down_score > 1.0 {
                self.down_score *= 0.921;
            }
        }
    }

    fn inc_down_score(&mut self, initiation_time: EpochTimestamp) {
        // Note: There is no 'dec_down_score()'. See 'inc_up_score()' for how
        //       the down_score is slowly reduced.
        if initiation_time > self.most_recent_initiated_timestamp {
            // First update the boolean state which is either healthy or not.
            self.is_healthy = false;
            self.most_recent_initiated_timestamp = initiation_time;

            // Every subsequent bad report gives a 21% penalty...
            if self.down_score < 1.0 {
                self.down_score = 1.0;
            } else if self.up_score < 100.0 {
                let new_value = self.down_score * 1.21;
                self.down_score = new_value.min(100.0);
            }

            // Slowly reduce the "up_score" (for next time it is up).
            if self.up_score > 1.0 {
                self.up_score *= 0.963;
            }
        }
    }
}

impl Default for ServerStats {
    fn default() -> Self {
        Self::new(String::default())
    }
}
