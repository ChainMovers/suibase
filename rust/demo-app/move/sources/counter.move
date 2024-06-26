// A shared object that controls an integer that can:
//    - Be incremented by anyone.
//    - Emits an event on change.
//    - Emits "console" log messages for VSCode debugging.
//
// There is only one Counter object created per package published.
//
module demo::Counter {        
    use sui::event;    

    use log::console::{Console};

    // Allow unit test module to use this object friend functions.
    //#[test_only]
    //public(package) demo::test_counter;

    // Move event emitted on every increment.
    public struct CounterChanged has copy, drop {
        count: u64,  // New value.
        by_address: address, // Sender of the transaction that caused the change.
    }

    // Shared object that is targeted for the demo.
    public struct Counter has key, store {
        id: UID,
        count: u64,
    }

    // The initialization function called at the moment of publication.
    fun init(ctx: &mut TxContext) {
      let new_counter = Counter { id: object::new(ctx), count: 0 };
      transfer::share_object( new_counter );
    }

    #[test_only]
    public(package) fun init_for_test( ctx: &mut TxContext)
    {
        init(ctx);
    }

    #[test_only]
    public(package) fun new( ctx: &mut TxContext): Counter
    {
        Counter { id: object::new(ctx), count: 0 }
    }

    public(package) fun delete( self: Counter )
    {
        let Counter { id, count: _ } =  self;
        object::delete(id);
    }

    public(package) fun count(self: &Counter): u64 {
        self.count
    }

    public(package) fun inc(self: &mut Counter, console: &Console, ctx: &TxContext)
    {                
        console.debug(b"internal inc() called");

        self.count = self.count + 1;

        let sender = tx_context::sender(ctx);
        event::emit( CounterChanged { count: self.count, by_address: sender } );
    }

    // Transaction to increment the counter
    public entry fun increment(self: &mut Counter, ctx: &TxContext)
    {
        // Initalize the console. Default is to enable all log levels.
        let mut console = log::console::default();
        log::console::set_log_level(&mut console, log::consts::Info());

        // Log a message.        
        console.info(b"increment() entry called");

        // No check of the sender. Anyone can increment the counter.
        demo::Counter::inc(self, &console, ctx);
    }
}

#[test_only]
module demo::test_counter {
    use sui::test_scenario::{Self};
    use demo::Counter::{Self};

    #[test]
    fun test_simple() {
        let creator = @0x1;
        let mut scenario_val = test_scenario::begin(creator);
        let scenario = &mut scenario_val;

        {
          Counter::init_for_test(test_scenario::ctx(scenario));
        };


        test_scenario::next_tx(scenario, creator);
        {
            let console = log::console::default();
            let ctx = test_scenario::ctx(scenario);

            let mut the_counter = Counter::new(ctx);
            assert!(Counter::count(&the_counter) == 0, 1);

            Counter::inc(&mut the_counter, &console, ctx);
            assert!(Counter::count(&the_counter) == 1, 1);

            transfer::public_share_object( the_counter );
        };

        test_scenario::end(scenario_val);
    }
}
