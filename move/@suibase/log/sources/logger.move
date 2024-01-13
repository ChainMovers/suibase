// Simple storage of configuration variables for a Console object.
module log::logger {    
    
    use sui::object::{Self, ID, UID};
    use std::vector::{Self};
    use sui::tx_context::{TxContext};
    use sui::transfer::{Self};
    

    use log::console_config::{Self,ConsoleConfig};
    use log::consts::{Self};

    friend log::init;
    friend log::console;
    friend log::logger_admin_cap;

    // One LevelStats for each log level + one for silent errors.
    public struct LevelStats has store, copy {
      count: u64, // Increment for every event emitted at this level.
    }

    public struct Logger has key, store {
        // Shared object singleton within this package.
        // Created once in init() of this module.
        // Controlled by LoggerAdminCap.
        id: UID,
        console_config: ConsoleConfig,

        // Stats while log is_enabled.
        //
        // Index == 0 is stats of silent errors.
        // Index >= 1 are stats for each log level.
        level_stats: vector<LevelStats>,

        // Number of time reset_stats() was done by a LoggerAdminCap.
        reset_stats_count: u64,        
    }

    public(friend) fun new( ctx: &mut TxContext ): ID {
        let console_config = console_config::new();
        let mut level_stats = vector::empty<LevelStats>();
        let mut i = 0u8;
        while (i < consts::MaxLogLevel()) {
            vector::push_back(&mut level_stats, LevelStats{count: 0});
            i = i + 1;
        };
        let reset_stats_count = 0;

        let new_logger = Logger {
            id: object::new(ctx),
            console_config,
            level_stats,
            reset_stats_count,
        };
        let ret_value = object::uid_to_inner(&new_logger.id);
        transfer::share_object( new_logger );
        ret_value
    }

    public(friend) fun id( self: &Logger): ID {
        object::uid_to_inner(&self.id)
    }

    public(friend) fun reset_stats( self: &mut Logger) {
        let mut i = 0u64;
        // Clear stats for all log levels (index '0' is stats of silent errors).
        let max_level = (log::consts::MaxLogLevel() as u64);
        while (i <= max_level) {
            let level_stats = vector::borrow_mut<LevelStats>(&mut self.level_stats, i);
            level_stats.count = 0;
            i = i + 1;
        };

        self.reset_stats_count = self.reset_stats_count + 1;
    }

    public(friend) fun set_log_level( self: &mut Logger, level: u8) {
        console_config::set_log_level(&mut self.console_config, level);
    }

    public(friend) fun enable( self: &mut Logger) {
        console_config::enable(&mut self.console_config);
    }

    public(friend) fun disable( self: &mut Logger) {
        console_config::disable(&mut self.console_config);
    }    
}
