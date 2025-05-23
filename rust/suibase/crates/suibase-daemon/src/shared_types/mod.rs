// This is for shared variables (used by more than one thread).
//
// This is a submodule specific to suibase-daemon.
//
// flatten everything under "shared_type" module.
pub(crate) use self::events::*;
pub(crate) use self::globals::*;
pub(crate) use self::input_port::*;
pub(crate) use self::packages::*;
pub(crate) use self::server_stats::*;
pub(crate) use self::target_server::*;

mod events;
mod globals;
mod input_port;
mod packages;
mod server_stats;
mod target_server;
