// Allow a move module to emit "console" events.
//
// UI expected to typically display the events sequentially as
// if a "console".
//
// Long-term goal is to add Rust-like macros like:
//  warn!(), info!(), debug!(), trace!()
//
// Example of use:
//   public entry fun user_function( ctx: &TxContext ) {
//      let console = log::console::default();
//      console.info("Hello World");
//   }
//
// Code requires edition 2024 of Sui Move:
//   https://github.com/MystenLabs/sui/issues/14062
//
module log::console {
    //use sui::object::{Self, ID};
    //use sui::tx_context::{TxContext};
    use sui::event;

    use std::string::{Self,String};
    use log::console_config::{Self,ConsoleConfig};
    use log::consts::{Self};

    struct ConsoleEvent has copy, drop {        
        level: u8,  // One of Error(1), Warn(2), Info(3), Debug(4) or Trace(5).
        message: String, // The message to log.
    }

    /*
    struct WatchEvent has copy, drop {
        // Watch are Key/Value where Value is a JSON object.
        //
        // The key/value are stored on-chain, and this event is 
        // emitted on value change.
        //
        // LoggerAdminCap can optionally filter objects to watch.
        key: String,
        value: String,
    }*/

    struct Console has drop {        
        // A user function either:
        //
        //  (1) Create a default Console object instance (no Logger object needed).
        //      
        //  (2) Create a Console object using a Logger object. The Logger defines the default
        //      behavior controled by the LoggerAdminCap.
        //
        // In all cases, the function can optionally customize the default behavior (e.g. with set_log_level).
        //
        // Example without Logger instance:
        //   public entry fun user_function( ctx: &TxContext ) {
        //      let console = log::console::default(); <-- Default to display all log levels.
        //      console.set_log_level(Info); <-- Optionally change default behavior.
        //      ...
        //      console.info("Hello World");          
        //      console.error("Sky is falling");
        //   }        
        //
        // Example with Logger instance:
        //   public entry fun user_function( logger: &mut Logger, ctx: &TxContext ) {
        //      let console = logger.console(ctx); <-- Default to what LoggerAdminCap configures.
        //      console.set_log_level(Info);  <-- Optionally change default behavior.
        //      ...
        //      console.info("Hello World"); 
        //      console.error("Sky is falling");
        //   }        
        config: ConsoleConfig,
    }

    // Create a console object by using one of the "default" function:
    //   default()
    //   default_error_only()
    //   default_disabled()
    //
    // After creation, the console behavior can be further adjusted 
    // with set_log_level(), enable() and disable().

    // Create a console the logs at all levels (Error, Warn, Info, Debug, Trace)        
    public fun default() : Console {        
        let config = console_config::new();
        Console { config }
    }

    // Create a console that logs only at Error level.
    //
    // Warn, Info, Debug and Trace levels have no effect.
    public fun default_error_only() : Console {
        let config = console_config::new();
        console_config::set_log_level(&mut config, consts::Error());
        Console { config }
    }    

    // Create a console with logging disabled.
    //
    // No events will be emitted.
    //
    // Error level will only increment the silent error stats if
    // a Logger object is used.
    //
    // Warn, Info, Debug and Trace levels have no effect.
    public fun default_disabled() : Console {
        let config = console_config::new();
        console_config::disable(&mut config);
        Console { config }
    }    


    public fun log( self: &Console, level: u8, message: vector<u8>) {
        if (!console_config::is_enabled(&self.config) || level > console_config::log_level(&self.config)) return;        
        let event = ConsoleEvent { level, message: string::utf8(message) };
        event::emit(event);
    }


    public fun error(self: &Console, message: vector<u8>) {
        log::console::log(self, consts::Error(), message);
    }

    public fun warn(self: &Console, message: vector<u8>) {
        log::console::log(self, consts::Warn(), message);
    }

    public fun info(self: &Console, message: vector<u8>) {
        log::console::log(self, consts::Info(), message);
    }

    public fun debug(self: &Console, message: vector<u8>) {
        log::console::log(self, consts::Debug(), message);
    }

    public fun trace(self: &Console, message: vector<u8>) {
        log::console::log(self, consts::Trace(), message);
    }

    public fun set_log_level(self: &mut Console, level: u8) {
        console_config::set_log_level(&mut self.config, level);
    }

    public fun enable(self: &mut Console) {
        console_config::enable(&mut self.config);
    }

    public fun disable(self: &mut Console) {
        console_config::disable(&mut self.config);
    }

    public fun is_enabled(self: &Console): bool {
        console_config::is_enabled(&self.config)
    }

}

// By default, the sui base scripts verify that all unit tests are passing prior
// to publication on non-local networks (e.g. when 'devnet publish').
#[test_only]
module log::test_console {
    // TODO !!!!!!!!!!!!!!
    use sui::transfer;
    use sui::test_scenario::{Self};
    use Counter::{Self};

    #[test]
    fun test_simple() {
        let creator = @0x1;
        let scenario_val = test_scenario::begin(creator);
        let scenario = &mut scenario_val;

        test_scenario::next_tx(scenario, creator);
        {
            let ctx = test_scenario::ctx(scenario);

            let the_counter = Counter::new(ctx);
            assert!(Counter::count(&the_counter) == 0, 1);

            Counter::inc(&mut the_counter, ctx);
            assert!(Counter::count(&the_counter) == 1, 1);

            transfer::share_object( the_counter );
        };

        test_scenario::end(scenario_val);
    }
}
