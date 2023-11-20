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

// Generic channel messages for simple coordination between tokio threads.
pub type GenericTx = tokio::sync::mpsc::Sender<GenericChannelMsg>;
pub type GenericRx = tokio::sync::mpsc::Receiver<GenericChannelMsg>;

// All events have an event_id field. Possible values are:
//
//       0 Undefined/Unknown ID.
//   1-127 reserved for GenericChannelEventID.
// 128-253 extension for specialize channels (e.g. AdminControllerEventID)
//     254 reserved for future.
//
pub type GenericChannelEventID = u8;
pub const EVENT_AUDIT: u8 = 1; // Fast consistency check. Globals read-only access. Should emit an EVENT_UPDATE to self for globals update.
pub const EVENT_UPDATE: u8 = 2; // Apply Globals config changes and/or update status. Globals write access allowed.
pub const EVENT_EXEC: u8 = 3; // Execute what is specified by the params. Globals write access allowed.

pub struct GenericChannelMsg {
    pub event_id: GenericChannelEventID,
    // Params
    pub data_string: Option<String>,
    pub workdir_idx: Option<WorkdirIdx>,

    // Optional channel to send a one-time response.
    pub resp_channel: Option<tokio::sync::oneshot::Sender<String>>,
}

impl GenericChannelMsg {
    pub fn new() -> Self {
        Self {
            event_id: 0,
            data_string: None,
            workdir_idx: None,
            resp_channel: None,
        }
    }
    pub fn data_string(&self) -> Option<String> {
        self.data_string.clone()
    }

    pub fn workdir_idx(&self) -> Option<WorkdirIdx> {
        self.workdir_idx
    }
}

impl std::fmt::Debug for GenericChannelMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericChannelMsg")
            .field("event_id", &self.event_id)
            .field("data_string", &self.data_string)
            .field("workdir_idx", &self.workdir_idx)
            .finish()
    }
}
