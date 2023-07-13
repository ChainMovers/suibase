// Maintains stats/health of a server (IP:Port).

use crate::basic_types::*;
pub struct ServerStats {
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
    avg_latency: u32,
    most_recent_latency_report: EpochTimestamp,

    // Total counts (never cleared).
    latency_report_count: u64,
    ok_count: u64,
    failed_count: u64,

    health_score: i8,
}

impl ServerStats {
    pub fn new() -> Self {
        let now = EpochTimestamp::now();
        Self {
            is_healthy: false,
            most_recent_initiated_timestamp: now,

            avg_latency: 0,
            most_recent_latency_report: now,

            latency_report_count: 0,
            ok_count: 0,
            failed_count: 0,

            health_score: 0,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    pub fn avg_latency(&self) -> u32 {
        self.avg_latency
    }

    pub fn most_recent_latency_report(&self) -> Option<EpochTimestamp> {
        if self.latency_report_count == 0 {
            return None;
        }
        Some(self.most_recent_latency_report)
    }

    pub fn report_ok(&mut self, initiation_time: EpochTimestamp) {
        if initiation_time > self.most_recent_initiated_timestamp {
            self.is_healthy = true;
            self.most_recent_initiated_timestamp = initiation_time;
        }
        self.ok_count += 1;
    }

    pub fn report_failed(&mut self, initiation_time: EpochTimestamp) {
        if initiation_time > self.most_recent_initiated_timestamp {
            self.is_healthy = false;
            self.most_recent_initiated_timestamp = initiation_time;
        }
        self.failed_count += 1;
    }

    pub fn report_latency(&mut self, initiation_time: EpochTimestamp, latency_microsecs: u32) {
        if self.latency_report_count == 0 {
            // One-time initialization
            self.most_recent_latency_report = initiation_time;
            self.avg_latency = latency_microsecs;
            self.latency_report_count = 1;
            return;
        }

        self.latency_report_count += 1;

        if initiation_time < self.most_recent_latency_report {
            // Out-of-order report, ignore it.
            return;
        }

        self.most_recent_latency_report = initiation_time;

        // Check for large value that would hint to a bug.
        let mut latency_microsecs = latency_microsecs;
        if latency_microsecs > MICROSECOND_LIMIT {
            latency_microsecs = MICROSECOND_LIMIT;
            log::error!("ServerStats::report_latency() clamped to 10 secs");
        }

        // Use a 20 measurements exponential moving average to smooth out the latency.
        let alpha = 0.05;
        self.avg_latency =
            (self.avg_latency as f32 * (1.0 - alpha) + latency_microsecs as f32 * alpha) as u32;
    }

    // A score from -100 to 100 about the relative health of
    // this server compare to other servers.
    //
    // TODO: The logic is not decided yet... and the relative part is WIP.
    //
    pub fn relative_health_score(&self) -> i8 {
        if self.is_healthy {
            100 // Ok
        } else if self.failed_count > 0 {
            -100 // Bad
        } else {
            0 // Neutral
        }
    }
}

impl Default for ServerStats {
    fn default() -> Self {
        Self::new()
    }
}
