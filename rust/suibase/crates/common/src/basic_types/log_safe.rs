// This singleton make sure that the same log message does not clutter the log file.
//
// Message from same caller location displayed within 1 minute since the last occurence
// are counted instead of being log.
//
use chrono::{Duration, Utc};
use log::info;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

struct LoggerState {
    last_log_time: Option<chrono::DateTime<Utc>>,
    counter: u32,
}

pub struct LogSafe {
    logger_states: Arc<Mutex<HashMap<String, Arc<Mutex<LoggerState>>>>>,
}

impl LogSafe {
    fn new() -> Self {
        LogSafe {
            logger_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn info(&self, msg: &str, file: &str, line: u32) {
        let caller = format!("{}:{}", file, line);
        let state = {
            let mut logger_states = self.logger_states.lock().await;
            logger_states
                .entry(caller.to_string())
                .or_insert_with(|| {
                    Arc::new(Mutex::new(LoggerState {
                        last_log_time: None,
                        counter: 0,
                    }))
                })
                .clone()
        };

        let mut state = state.lock().await;

        let now = Utc::now();
        match state.last_log_time {
            Some(last_time) if (now - last_time) < Duration::minutes(1) => {
                // If it's been less than a minute since the last log, increment the counter
                state.counter += 1;
            }
            _ => {
                // If it's been more than a minute since the last log or if this is the first log,
                // log the counter (if this isn't the first log), reset the counter and update the last log time
                if state.counter > 0 {
                    info!("(repeat {}) caller=[{}] [{}]", state.counter, caller, msg);
                } else {
                    info!("caller=[{}] [{}]", caller, msg);
                }
                state.counter = 0;
                state.last_log_time = Some(now);
            }
        }
    }
}

pub static LOG_SAFE: Lazy<LogSafe> = Lazy::new(LogSafe::new);

#[macro_export]
macro_rules! log_safe {
    ($msg:expr) => {
        $crate::basic_types::LOG_SAFE
            .info(&format!("{}", $msg), file!(), line!())
            .await;
    };
}
