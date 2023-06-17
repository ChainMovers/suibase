// Maintains stats/health of a server (IP:Port).

// TODO: Very basic for now. Intended to eventually expand with duration, logs, latency avg etc...
use crate::basic_types::*;
pub struct ServerStats {
    is_healthy: bool,
    last_refresh: EpochTimestamp,
    last_transition: EpochTimestamp,
    last_latency: u32,
    health_score: i8,
}

impl ServerStats {
    pub fn new() -> Self {
        let last_refresh = EpochTimestamp::now();
        Self {
            is_healthy: false,
            last_refresh,
            last_transition: last_refresh,
            last_latency: 0,
            health_score: 0,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    pub fn report_ok(&mut self) {
        self.last_refresh = EpochTimestamp::now();
        if !self.is_healthy {
            self.is_healthy = true;
            self.last_transition = self.last_refresh;
        }
    }

    pub fn report_failed(&mut self) {
        self.last_refresh = EpochTimestamp::now();
        if self.is_healthy {
            self.is_healthy = false;
            self.last_transition = self.last_refresh;
        }
    }

    pub fn report_latency(&mut self, latency_ms: u32) {
        self.report_ok();
        self.last_latency = latency_ms;
    }

    // A score from -100 to 100 about the relative health of
    // this server compare to other servers.
    //
    // The default is "0" (unknown/neutral).
    //
    // TODO: The score slowly adjusts with time when update_relative_health_score() is called.
    //
    pub fn relative_health_score(&self) -> i8 {
        return 0;
    }
}
