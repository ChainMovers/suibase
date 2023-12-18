// Simple storage of configuration variables for a Console object.
module log::console_config {    
    
    // Log levels.
    #[allow(unused_const)]
    const ERROR_LEVEL: u8 = 1;
    const WARN_LEVEL: u8 = 2;
    const INFO_LEVEL: u8 = 3;
    const DEBUG_LEVEL: u8 = 4;
    const TRACE_LEVEL: u8 = 5;

    // Use thin function until public enum is supported...
    public fun Error() : u8 { ERROR_LEVEL }
    public fun Warn()  : u8 { WARN_LEVEL }
    public fun Info()  : u8 { INFO_LEVEL }
    public fun Debug() : u8 { DEBUG_LEVEL }
    public fun Trace() : u8 { TRACE_LEVEL }

    /***********************************************************************
     * ConsoleConfig
     ***********************************************************************/
    public struct ConsoleConfig has drop, store {
        enabled: bool,
        log_level: u8,
    }

    public fun enable( self: &mut ConsoleConfig ) {
        self.enabled = true;
    }

    public fun disable( self: &mut ConsoleConfig ) {
        self.enabled = false;
    }

    public fun enabled( self: &ConsoleConfig ) : bool {
        self.enabled
    }

    public fun set_log_level( self: &mut ConsoleConfig, level: u8 ) {
        self.log_level = level;
    }

    public fun log_level(self: &ConsoleConfig) : u8 {
        self.log_level
    }

    public fun new() : ConsoleConfig {
        ConsoleConfig { enabled: true, log_level: TRACE_LEVEL }
    }
}
