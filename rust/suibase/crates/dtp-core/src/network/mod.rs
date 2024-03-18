// Flatten many sub modules/files under the same dtp_core::network module.
//
// Allows to do:
//    use dtp_core::network::{NetworkManager, HostInternal, LocalhostInternal}
//
// Instead of verbose:
//    use dtp_core::network::NetworkManager;
//    use dtp_core::network::host_internal::HostInternal;
//    use dtp_core::network::localhost_internal::LocalhostInternal;
//pub use self::common_rpc::*;
pub use self::host_internal::*;
pub use self::localhost_internal::*;
pub use self::network_manager::*;
pub use self::serde_types::*;
pub use self::transport_control_internal::*;
pub use self::user_registry::*;

mod common_rpc;
mod host_internal;
mod localhost_internal;
mod network_manager;
mod serde_types;
mod transport_control_internal;
mod user_registry;
