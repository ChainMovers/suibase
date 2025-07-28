// This is for shared variables (used by more than one thread).
//
// Type defined here are expected to be made MT safe with Arc::Mutex.
//
// Flattens everything under "common::shared_type" module.
pub use self::workdirs::*;

pub mod workdirs;
