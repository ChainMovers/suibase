module log::init {
    //use sui::object::{Self};
    use sui::tx_context::{TxContext};

    use log::logger::{Self};
    use log::logger_admin_cap::{Self};

    // A convenient default initialization. 
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
      let new_logger_id = logger::new( ctx );

      // Create the initial logger administrator (more can be added later for fallback).      
      logger_admin_cap::new(new_logger_id, ctx);      
    }
}