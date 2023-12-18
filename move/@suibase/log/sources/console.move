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
    use sui::object::{Self, ID, UID};
    use sui::tx_context::{TxContext};
    use sui::event;
    use sui::transfer;
    use std::string::{String};
    use log::console_config::{Self,ConsoleConfig, Error, Warn, Info, Debug, Trace};

    // Error codes.
    const ENotLogCapOwner: u64 = 1;
    const EOutOfRangeLogLevel: u64 = 2;


    public struct ConsoleEvent has copy, drop {        
        level: u8,  // One of Error(1), Warn(2), Info(3), Debug(4) or Trace(5).
        message: String, // The message to log.
    }

    public struct WatchEvent has copy, drop {
        // Watch are Key/Value where Value is a JSON object.
        //
        // The key/value are stored on-chain, and this event is 
        // emitted on value change.
        //
        // LoggerAdminCap can optionally filter objects to watch.
        key: String,
        value: String,
    }
    
    // By default log everything (Error, Warn, Info, Debug, Trace)
    // Caller can further change the configuration with set_log_level().
    public fun default() : Console {
        // console_config default is Trace level.
        let config = console_config::new();
        Console { config }
    }

    // Another default that display only Error levels 
    // Warn, Info, Debug, Trace will have no effect.
    public fun default_error_only() : Console {
        let mut config = console_config::new();
        config.set_log_level(console_config::Error());
        Console { config }
    }    

    /****************
     * Console
     ***************/
    public struct Console has drop {        
        // A user function either:
        //
        //  (1) Create a default Console object instance (no Logger object needed).
        //      
        //  (2) Create a Console object using a Logger object. The Logger defines the default
        //      behavior controled by the LoggerAdminCap.
        //
        // In both case, the function can optionally customize the default behavior (e.g. with set_log_level).
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
        //   public entry fun user_function( logger: Logger, ctx: &TxContext ) {
        //      let console = logger.console(ctx); <-- Default to what LoggerAdminCap configures.
        //      console.set_log_level(Info);  <-- Optionally change default behavior.
        //      ...
        //      console.info("Hello World"); 
        //      console.error("Sky is falling");
        //   }        
        config: ConsoleConfig,
    }

    public fun log( self: &Console, level: u8, message: String ) {
        let config = &self.config;
        if (!config.enabled()) return;
        if (level > self.config.log_level()) return;
        let event = ConsoleEvent { level, message };
        event::emit(event);
    }


    public fun error(self: &Console, message: String) {
        self.log(Error(), message);
    }

    public fun warn(self: &Console, message: String) {
        self.log(Warn(), message);
    }

    public fun info(self: &Console, message: String) {
        self.log(Info(), message);
    }

    public fun debug(self: &Console, message: String) {
        self.log(Debug(), message);
    }

    public fun trace(self: &Console, message: String) {
        self.log(Trace(), message);
    }

/* TODO Move LoggerAdminCap in seperate Module 
    public fun set_log_level( self: &mut Console, log_level: u8, _ctx: &TxContext ) {
        // Validate log_level is in range 1..5
        assert!(log_level >= 1 && log_level <= 5, EOutOfRangeLogLevel);
        self.config.set_log_level(log_level);
    }

    public fun enable( self: &mut Console, _ctx: &TxContext ) {
        self.config.enable();
    }

    public fun disable( self: &mut Console, _ctx: &TxContext ) {
        self.config.disable();
    }
*/
    /***********************************************************************
     * Logger
     ***********************************************************************/
    public struct Logger has key {
        // Shared object singleton within this package.
        // Created once in init() of this module.
        // Controlled by LoggerAdminCap.
        id: UID,
        console_config: ConsoleConfig,
    }

    // Allow unit test module to use this object friend functions.
    #[test_only]
    friend log::test_console;


    /***********************************************************************
     * LoggerAdminCap
     ***********************************************************************/
    public struct LoggerAdminCap has key {        
        id: UID,
        owner: address,
        logger_id: ID,        
    }

    public fun transfer( mut self: LoggerAdminCap, new_owner: address, ctx: &TxContext) {
        // TODO Add to a registry for easier management.
        // The owner can transfer only its own LoggerAdminCap.
        assert!(ctx.sender() == self.owner, ENotLogCapOwner);
        self.owner = new_owner;
        sui::transfer::transfer(self, new_owner);
    }

    public fun enable_console( self: &LoggerAdminCap, logger: &mut Logger, _ctx: &TxContext) {
        assert!(self.logger_id == logger.id.uid_to_inner(), ENotLogCapOwner);
        logger.console_config.enable();
    }

    public fun disable_console( self: &LoggerAdminCap, logger: &mut Logger, _ctx: &TxContext) {
        assert!(self.logger_id == logger.id.uid_to_inner(), ENotLogCapOwner);
        logger.console_config.disable();
    }

    // A conveninent default initialization. 
    //
    // Intended to be called from init() of the package.
    //
    // Will be sufficient for most users.
    fun init(ctx: &mut TxContext) {
      // Everyone can use the singleton Logger shared object 
      // and the LoggerAdminCap owner controls its behavior.
      // 
      // TODO Verify the following assumption:
      //
      // Keep in mind that many instances of this log module will be published
      // on the network, but they will be within different packages instance. 
      // Therefore, this Logger instance is a unique type within this package
      // and will not mix/interfere with any other Logger instance in other packages.      
      let id = object::new(ctx);
      let logger_id = id.uid_to_inner();      
      let logger = Logger { id, console_config: console_config::new() };
      transfer::share_object( logger );

      // Create the initial logger administrator (more can be added later for fallback).
      let id = object::new(ctx);              
      sui::transfer::transfer(LoggerAdminCap { id, owner: ctx.sender(), logger_id }, ctx.sender());
    }

    public entry fun set_log_level( self: &mut LoggerAdminCap, logger: &mut Logger, log_level: u8, _ctx: &TxContext ) {    
        assert!(self.logger_id == logger.id.uid_to_inner(), ENotLogCapOwner);
        // Validate log_level is in range 1..5
        assert!(log_level >= 1 && log_level <= 5, EOutOfRangeLogLevel);
        logger.console_config.set_log_level(log_level);
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
