// A shared object that controls an integer that can:
//    - Be incremented by anyone.
//    - Emits an event on change.
//
// There is only one Counter object created per package published.
//
// That object instance can be found by any clients through a RPC query.
// 
// TODO Modify to use template type so it is not so trivial may be?
// TODO Add capability of clearing only from creator?
module demo::Counter {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self,TxContext};
    use sui::event;
    use sui::transfer;
    
    // Allow unit test module to use this object friend functions.
    #[test_only]
    friend demo::test_counter;

    // Move event emitted on every increment.
    struct CounterChanged has copy, drop {
        count: u64,  // New value.
        by_address: address, // Sender of the transaction that caused the change.
    }

    // Shared object that is targeted for the demo.
    struct Counter has key, store {
        id: UID,
        count: u64,
    }

    // The initialization function called at the moment of publication.
    fun init(ctx: &mut TxContext) { 
      let new_counter = demo::Counter::new(ctx);
      transfer::share_object( new_counter );
    }

    // Notice that for this example, the new() is not called from 
    // a transaction.
    //
    // Only init() calls new to guarantee one instance per package.
    //
    // It is still mark with (friend) to allow for unit testing.
    public fun new( ctx: &mut TxContext): Counter
    {
        Counter { id: object::new(ctx), count: 0 }
    }

    public(friend) fun delete( self: Counter )
    {
        let Counter { id, count: _ } =  self;
        object::delete(id);
    }

    public(friend) fun count(self: &Counter): u64 {
        self.count
    }

    public(friend) fun inc(self: &mut Counter, ctx: &TxContext)
    {
        self.count = self.count + 1;

        let sender = tx_context::sender(ctx);  
        event::emit( CounterChanged { count: self.count, by_address: sender } );
    }

    // Transaction to increment the counter
    public entry fun increment(self: &mut Counter, ctx: &TxContext)
    {
        // No check of the sender. Anyone can increment the counter.
        demo::Counter::inc(self, ctx);
    }
}

// By default, the sui base scripts verify that all unit tests are passing prior
// to publication on non-local networks (e.g. when 'devnet publish').
#[test_only]
module demo::test_counter {
    use sui::transfer;
    use sui::test_scenario::{Self};
    use demo::Counter::{Self};

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
