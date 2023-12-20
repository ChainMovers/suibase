// Simple storage of configuration variables for a Console object.
module log::logger_admin_cap {    
    
    use sui::object::{Self, ID, UID};
    use sui::tx_context::{Self,TxContext};

    use log::consts::{Self};
    use log::logger::{Self, Logger};

    friend log::init;

    struct LoggerAdminCap has key {
        id: UID,
        owner: address,
        logger_id: ID,        
    }

    #[lint_allow(self_transfer)]
    public(friend) fun new( logger_id: ID, ctx: &mut TxContext) {
        let new_cap = LoggerAdminCap {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            logger_id: logger_id,
        };
        sui::transfer::transfer( new_cap, tx_context::sender(ctx));
    }

    #[lint_allow(self_transfer)]
    public entry fun transfer( self: LoggerAdminCap, new_owner: address, ctx: &mut TxContext) {
        // TODO Add to a registry for easier management.
        // The owner can transfer only its own LoggerAdminCap.
        assert!(tx_context::sender(ctx) == self.owner, consts::ENotLogCapOwner());
        assert!(self.owner != new_owner, consts::ETransferToSelf());

        self.owner = new_owner;
        sui::transfer::transfer(self, new_owner);
    }

    public entry fun enable_console( self: &LoggerAdminCap, logger: &mut Logger, _ctx: &mut TxContext) {
        assert!(self.logger_id == logger::id(logger), consts::ENotLogCapOwner());
        logger::enable(logger);
    }

    public entry fun disable_console( self: &LoggerAdminCap, logger: &mut Logger, _ctx: &mut TxContext) {
        assert!(self.logger_id == logger::id(logger), consts::ENotLogCapOwner());
        logger::disable(logger);
    }

    public entry fun reset_stats( self: &LoggerAdminCap, logger: &mut Logger, _ctx: &mut TxContext) {
        assert!(self.logger_id == logger::id(logger), consts::ENotLogCapOwner());
        logger::reset_stats(logger);
    }

    public entry fun set_log_level( self: &LoggerAdminCap, logger: &mut Logger, log_level: u8, _ctx: &mut TxContext ) {  
        assert!(self.logger_id == logger::id(logger), consts::ENotLogCapOwner());
        logger::set_log_level(logger, log_level);
    }
}
