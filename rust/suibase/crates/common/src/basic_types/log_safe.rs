// This singleton make sure that the same log message does not clutter the log file.
//
// Message from same caller location displayed within 1 minute since the last occurence
// are counted instead of being log.
//
use chrono::{Duration, Utc};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
pub struct TracingSafe {
    logger_states: Arc<std::sync::Mutex<HashMap<String, LoggerState>>>,
}

struct LoggerState {
    last_log_time: Option<chrono::DateTime<Utc>>,
    counter: u32,
}

impl TracingSafe {
    fn new() -> Self {
        TracingSafe {
            logger_states: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    // Make synchronous for easier use in macros
    pub fn log_safe(&self, level: tracing::Level, msg: &str, file: &str, line: u32) {
        // Get just the filename
        let file = Path::new(file)
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("unknown");

        // Create a target that includes only file and line
        let location = format!("{}:{}", file, line);

        // Add the level for deduplication.
        let caller = format!("{}:{}", level, location);

        // Use std::sync::Mutex
        let mut logger_states = match self.logger_states.lock() {
            Ok(guard) => guard,
            Err(_) => {
                error!("Failed to acquire lock for log deduplication");
                return;
            }
        };

        let state = logger_states.entry(caller.clone()).or_insert(LoggerState {
            last_log_time: None,
            counter: 0,
        });

        let now = Utc::now();
        match state.last_log_time {
            Some(last_time) if (now - last_time) < Duration::minutes(1) => {
                // If less than a minute, increment counter
                state.counter += 1;
            }
            _ => {
                // Log with tracing, including structured fields
                match level {
                    tracing::Level::ERROR => {
                        error!(
                            target: "safe",
                            "{} [{}]{}{}",
                            msg,
                            location,
                            if state.counter > 0 { "(" } else { "" },
                            if state.counter > 0 { state.counter.to_string() + ")" } else { "".to_string() },
                        )
                    }
                    tracing::Level::WARN => {
                        warn!(
                            target: "safe",
                            "{} [{}]{}{}",
                            msg,
                            location,
                            if state.counter > 0 { "(" } else { "" },
                            if state.counter > 0 { state.counter.to_string() + ")" } else { "".to_string() },
                        )
                    }
                    tracing::Level::DEBUG => {
                        debug!(
                            target: "safe",
                            "{} [{}]{}{}",
                            msg,
                            location,
                            if state.counter > 0 { "(" } else { "" },
                            if state.counter > 0 { state.counter.to_string() + ")" } else { "".to_string() },
                        )
                    }
                    _ => {
                        info!(
                            target: "safe",
                            "{} [{}]{}{}",
                            msg,
                            location,
                            if state.counter > 0 { "(" } else { "" },
                            if state.counter > 0 { state.counter.to_string() + ")" } else { "".to_string() },
                        )
                    }
                }

                // Reset state
                state.counter = 0;
                state.last_log_time = Some(now);
            }
        }
    }
}

// Singleton instance
pub static TRACING_SAFE: Lazy<TracingSafe> = Lazy::new(TracingSafe::new);

// Synchronous macros (no await needed)
#[macro_export]
macro_rules! log_safe {
    ($msg:expr) => {
        $crate::basic_types::TRACING_SAFE.log_safe(
            tracing::Level::INFO,
            &format!("{}", $msg),
            file!(),
            line!(),
        );
    };
}

#[macro_export]
macro_rules! log_safe_warn {
    ($msg:expr) => {
        $crate::basic_types::TRACING_SAFE.log_safe(
            tracing::Level::WARN,
            &format!("{}", $msg),
            file!(),
            line!(),
        );
    };
}

#[macro_export]
macro_rules! log_safe_err {
    ($msg:expr) => {
        $crate::basic_types::TRACING_SAFE.log_safe(
            tracing::Level::ERROR,
            &format!("{}", $msg),
            file!(),
            line!(),
        );
    };
}

#[macro_export]
macro_rules! log_safe_debug {
    ($msg:expr) => {
        $crate::basic_types::TRACING_SAFE.log_safe(
            tracing::Level::DEBUG,
            &format!("{}", $msg),
            file!(),
            line!(),
        );
    };
}

// A macro that check if a MPSC channel has more element queued
// than the threshold. When exceeding, display a message using
// a safe logger.
#[macro_export]
macro_rules! mpsc_q_check {
    ($param:expr) => {
        if $param.len() > $crate::basic_types::MPSC_Q_THRESHOLD {
            $crate::basic_types::TRACING_SAFE.log_safe(
                tracing::Level::INFO,
                &format!("{:?} len={}", $param, $param.len()),
                file!(),
                line!(),
            );
        }
    };
}
