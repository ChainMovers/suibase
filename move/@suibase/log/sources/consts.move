// Log levels.
//
// When log levels are used as index into a vector, the 
// value zero is intended for when logs are disabled.  
//
module log::consts {

  #[allow(unused_const)]
  const ERROR_LEVEL: u8 = 1;
  const WARN_LEVEL: u8 = 2;
  const INFO_LEVEL: u8 = 3;
  const DEBUG_LEVEL: u8 = 4;
  const TRACE_LEVEL: u8 = 5;

  const MIN_LOG_LEVEL: u8 = ERROR_LEVEL;
  const MAX_LOG_LEVEL: u8 = TRACE_LEVEL;

  // Use thin functions until public enum is supported...
  public fun Error() : u8 { ERROR_LEVEL }
  public fun Warn()  : u8 { WARN_LEVEL }
  public fun Info()  : u8 { INFO_LEVEL }
  public fun Debug() : u8 { DEBUG_LEVEL }
  public fun Trace() : u8 { TRACE_LEVEL }

  // Consts useful for iterating.
  public fun MinLogLevel() : u8 { MIN_LOG_LEVEL }
  public fun MaxLogLevel() : u8 { MAX_LOG_LEVEL }

  // Error codes.
  const E_NOT_LOG_CAP_OWNER: u64 = 1;
  const E_OUT_OF_RANGE_LOG_LEVEL: u64 = 2;
  const E_TRANSFER_TO_SELF: u64 = 3;

  // Use thin functions until public enum is supported...
  public fun ENotLogCapOwner() : u64 { E_NOT_LOG_CAP_OWNER }
  public fun EOutOfRangeLogLevel() : u64 { E_OUT_OF_RANGE_LOG_LEVEL }
  public fun ETransferToSelf() : u64 { E_TRANSFER_TO_SELF }

}