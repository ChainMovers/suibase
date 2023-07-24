// Some common types depending only on built-in or "standard" types.
pub type EpochTimestamp = tokio::time::Instant;

/*
use std::sync::atomic::{AtomicUsize, Ordering};

pub type InstanceID = usize;
pub fn gen_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}*/

// Some duration are stored in micro-seconds. In many context,
// above 1 min is a likely bug (with the benefit that the limit
// can be stored into 32-bits without failure).
pub const MICROSECOND_LIMIT: u32 = 60000000; // 1 minute
pub fn duration_to_micros(value: std::time::Duration) -> u32 {
    match value.as_micros().try_into() {
        Ok(value) => std::cmp::min(value, MICROSECOND_LIMIT),
        Err(_) => MICROSECOND_LIMIT,
    }
}

pub type InputPortIdx = crate::basic_types::ManagedVecUSize;
pub type TargetServerIdx = crate::basic_types::ManagedVecUSize;
pub type WorkdirIdx = crate::basic_types::ManagedVecUSize;
