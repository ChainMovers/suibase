// This is for shared variables (used by more than one thread).
//
// This is a submodule specific to suibase-daemon.
//
// flatten everything under "shared_type" module.
pub(crate) use self::channels::*;
pub(crate) use self::dtp_conns_state_client::*;
pub(crate) use self::dtp_conns_state_rx::*;
pub(crate) use self::dtp_conns_state_tx::*;
pub(crate) use self::events::*;
pub(crate) use self::globals::*;
pub(crate) use self::input_port::*;
pub(crate) use self::packages::*;
pub(crate) use self::server_stats::*;
pub(crate) use self::target_server::*;
pub(crate) use self::uuid::*;

mod channels;
mod dtp_conns_state_client;
mod dtp_conns_state_rx;
mod dtp_conns_state_tx;
mod events;
mod globals;
mod input_port;
mod packages;
mod server_stats;
mod target_server;
mod uuid;
