// Some common types depending only on built-in or "standard" types.
use std::sync::atomic::{AtomicUsize, Ordering};
pub type EpochTimestamp = tokio::time::Instant;
pub type IPKey = String;

// Internal (in-process) Unique ID.
pub type PortMapID = usize;
pub type MidClientID = usize;

pub fn gen_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
