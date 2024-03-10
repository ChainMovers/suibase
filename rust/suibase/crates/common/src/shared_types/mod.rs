// This is for shared variables (used by more than one thread).
//
// This is a submodule specific to suibase-daemon.
//
// flatten everything under "shared_type" module.
pub use self::workdirs::*;

mod workdirs;
