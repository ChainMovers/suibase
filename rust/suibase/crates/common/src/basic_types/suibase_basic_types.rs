// Some common types depending only on built-in or tokio types.
pub type EpochTimestamp = tokio::time::Instant;

// Event Level (matches  consts used in the Suibase log Move module)
pub type EventLevel = u8;
pub const EVENT_LEVEL_INVALID: u8 = 0u8;
pub const EVENT_LEVEL_ERROR: u8 = 1u8;
pub const EVENT_LEVEL_WARN: u8 = 2u8;
pub const EVENT_LEVEL_INFO: u8 = 3u8;
pub const EVENT_LEVEL_DEBUG: u8 = 4u8;
pub const EVENT_LEVEL_TRACE: u8 = 5u8;
pub const EVENT_LEVEL_MIN: u8 = EVENT_LEVEL_ERROR;
pub const EVENT_LEVEL_MAX: u8 = EVENT_LEVEL_TRACE;

pub const MPSC_Q_SIZE: usize = 200;
pub const MPSC_Q_THRESHOLD: usize = 150; // Will log an error if queue reaching this level.

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

pub type InputPortIdx = super::ManagedVecU8;
pub type TargetServerIdx = super::ManagedVecU8;
pub type WorkdirIdx = super::ManagedVecU8;

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
pub const EVENT_EXEC: u8 = 3; // Execute what is specified by the params (command, data_string...). Globals write access allowed.

#[derive(Default)]
pub struct GenericChannelMsg {
    pub event_id: GenericChannelEventID,

    // Params
    pub command: Option<String>,
    pub params: Vec<String>,

    pub data_json: Option<serde_json::Value>,
    pub workdir_idx: Option<WorkdirIdx>,

    // Optional channel to send a one-time response.
    pub resp_channel: Option<tokio::sync::oneshot::Sender<String>>,
}

impl Clone for GenericChannelMsg {
    fn clone(&self) -> Self {
        Self {
            event_id: self.event_id,
            command: self.command.clone(),
            params: self.params.clone(),
            data_json: self.data_json.clone(),
            workdir_idx: self.workdir_idx,
            resp_channel: None, // Watch-out... resp_channel is not cloneable!
        }
    }
}

impl GenericChannelMsg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn command(&self) -> Option<String> {
        self.command.clone()
    }

    pub fn params(&self, index: usize) -> Option<String> {
        self.params.get(index).cloned()
    }

    pub fn workdir_idx(&self) -> Option<WorkdirIdx> {
        self.workdir_idx
    }
}

impl std::fmt::Debug for GenericChannelMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericChannelMsg")
            .field("event_id", &self.event_id)
            .field("command", &self.command)
            .field("params", &self.params)
            .field("workdir_idx", &self.workdir_idx)
            .finish()
    }
}

pub struct AdminControllerMsg {
    // Message sent toward the AdminController from various sources.
    pub event_id: AdminControllerEventID,
    pub workdir_idx: Option<WorkdirIdx>,
    pub data_string: Option<String>,
    // Channel to send a one-time response.
    pub resp_channel: Option<tokio::sync::oneshot::Sender<String>>,
}

impl AdminControllerMsg {
    pub fn new() -> Self {
        Self {
            event_id: 0,
            workdir_idx: None,
            data_string: None,
            resp_channel: None,
        }
    }
    pub fn data_string(&self) -> Option<String> {
        self.data_string.clone()
    }
}

impl std::fmt::Debug for AdminControllerMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminControllerMsg")
            .field("event_id", &self.event_id)
            .field("data_string", &self.data_string)
            .finish()
    }
}

// Events ID
pub type AdminControllerEventID = u8;
pub const EVENT_NOTIF_CONFIG_FILE_CHANGE: u8 = 128;
pub const EVENT_DEBUG_PRINT: u8 = 129;
pub const EVENT_SHELL_EXEC: u8 = 130;
pub const EVENT_POST_PUBLISH: u8 = 131;

pub type AdminControllerTx = tokio::sync::mpsc::Sender<AdminControllerMsg>;
pub type AdminControllerRx = tokio::sync::mpsc::Receiver<AdminControllerMsg>;

// For struct that can be instantiated with a single parameter.
pub trait Instantiable<P> {
    fn new(params: P) -> Self;
}

// For struct that are guaranteed to have a WorkdirIdx field.
pub trait WorkdirContext {
    fn workdir_idx(&self) -> WorkdirIdx;
}
