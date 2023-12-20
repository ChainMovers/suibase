// Simple storage of configuration variables for a Console object.
module log::console_config {    
    use log::consts::{Self};

    struct ConsoleConfig has drop, store {
        is_enabled: bool,
        log_level: u8,
    }

    public fun new() : ConsoleConfig {
        ConsoleConfig { is_enabled: true, log_level: consts::MaxLogLevel() }
    }

    public fun is_enabled( self: &ConsoleConfig ) : bool {
        self.is_enabled
    }

    public fun log_level(self: &ConsoleConfig) : u8 {
        self.log_level
    }

    public fun enable( self: &mut ConsoleConfig ) {
        self.is_enabled = true;
    }

    public fun disable( self: &mut ConsoleConfig ) {
        self.is_enabled = false;
    }

    public fun set_log_level( self: &mut ConsoleConfig, level: u8 ) {
        assert!(level <= consts::MaxLogLevel(), consts::EOutOfRangeLogLevel());
        assert!(level >= consts::MinLogLevel(), consts::EOutOfRangeLogLevel());
        self.log_level = level;
    }
}
