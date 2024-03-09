// Flatten many sub modules/files under the same dtp_core::types module.
//
// Allows to do:
//    use dtp_core::types::{DTPError, PingStats}
//
// Instead of verbose:
//    use dtp_core::types::error::DTPError;
//    use dtp_core::types::stats::PingStats;
//    ...

pub use self::error::*;
pub use self::stats::*;
pub use self::sui_sdk_wrapped::*;

pub mod error;
pub mod stats;
pub mod sui_sdk_wrapped;
