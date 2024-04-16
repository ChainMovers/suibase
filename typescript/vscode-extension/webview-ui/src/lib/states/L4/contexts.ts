// Contexts are UI selectable to allow the user target a very different blockchain.
//
// Example:
//         Sui Network (Localnet)
//         Sui Network (Testnet)
//         Aptos Network (Localnet)
//         ....
//
// The webapp is displaying/background processing only in one context at the time.
//
import {
  readable,
  writable,
  get,
  type Writable,
  type Readable,
  type Subscriber,
  type Unsubscriber,
  type Updater,
} from "svelte/store";

// Layer 1
import { MSUI_CONTEXT_KEY, LSUI_CONTEXT_KEY, TSUI_CONTEXT_KEY, DSUI_CONTEXT_KEY } from "../L1/consts";
import type { IContextKeyed, IEpochETA } from "../L1/poc-interfaces";
import type { EpochLatest, EpochLeaderboard, EpochValidators } from "../L1/json-constructed";
import { LoadStore } from "../L1/stores";

// Layer 2
import type {
  IBlockchainContext,
  IBlockchainContextMapping,
  IBlockchainStores,
  IEpochStores,
  IGlobalsStores,
} from "../L2/interfaces";

// Layer 3
import { EpochStores } from "../L3/epoch_stores";
import { GlobalsStores } from "../L3/uuid_stores";

// Layer 5
/*import { StateLoop } from '$lib/states/states_loop'*/
/*
class SC_Context implements IBlockchainContext {
    readonly ui_selector_name = 'Safecoin Mainnet';
    readonly ui_name = 'Safecoin';
    readonly symbol = 'SAFE';
    readonly prefix = SC_CONTEXT_KEY;
    readonly server = 'ssc';
}

class TST_Context implements IBlockchainContext {
    readonly ui_selector_name = 'Safecoin Testnet';
    readonly ui_name = 'Safecoin';
    readonly symbol = 'SAFE';
    readonly prefix = TST_CONTEXT_KEY;
    readonly server = 'ssc';
}
*/

class LSUI_Context implements IBlockchainContext {
  readonly ui_selector_name = "Localnet";
  readonly ui_name = "Sui";
  readonly symbol = "Sui";
  readonly prefix = LSUI_CONTEXT_KEY;
  readonly workdir = "localnet";
  readonly server = "http://0.0.0.0:44399";
}

class TSUI_Context implements IBlockchainContext {
  readonly ui_selector_name = "Testnet";
  readonly ui_name = "Sui";
  readonly symbol = "Sui";
  readonly prefix = TSUI_CONTEXT_KEY;
  readonly workdir = "testnet";
  readonly server = "http://0.0.0.0:44399";
}

class DSUI_Context implements IBlockchainContext {
  readonly ui_selector_name = "Devnet";
  readonly ui_name = "Sui";
  readonly symbol = "Sui";
  readonly prefix = DSUI_CONTEXT_KEY;
  readonly workdir = "devnet";
  readonly server = "http://0.0.0.0:44399";
}

class MSUI_Context implements IBlockchainContext {
  readonly ui_selector_name = "Mainnet";
  readonly ui_name = "Sui";
  readonly symbol = "Sui";
  readonly prefix = MSUI_CONTEXT_KEY;
  readonly workdir = "mainnet";
  readonly server = "http://0.0.0.0:44399";
}

// Default initialization  value.
//
// One day, may be, this will be user adaptable (e.g. only dev account will be able to switch to the TST_Context)
//
//const _sc_context_obj = new SC_Context();
//const _tst_context_obj = new TST_Context();
const _LSUI_Context_obj = new LSUI_Context();
const _TSUI_Context_obj = new TSUI_Context();
const _DSUI_Context_obj = new DSUI_Context();
const _MSUI_context_obj = new MSUI_Context();

class BlockchainStores implements IBlockchainStores {
  readonly epoch_stores: IEpochStores;
  readonly globals_stores: IGlobalsStores;
  constructor(p_context: IBlockchainContext) {
    this.epoch_stores = new EpochStores(p_context);
    this.globals_stores = new GlobalsStores(p_context);
  }
}

class BlockchainContextMapping implements IBlockchainContextMapping {
  readonly context: IBlockchainContext;
  readonly stores: IBlockchainStores;
  constructor(p_context: IBlockchainContext) {
    this.context = p_context;
    this.stores = new BlockchainStores(this.context);
  }
}

// **WATCH OUT** Sui Localnet is default, make sure it match X_CONTEXT_KEY below.
const _default_context_map = new BlockchainContextMapping(_LSUI_Context_obj);

const _contexts_map = new Map<string, BlockchainContextMapping>([
  [_default_context_map.context.prefix, _default_context_map],
  [_TSUI_Context_obj.prefix, new BlockchainContextMapping(_TSUI_Context_obj)],
  [_DSUI_Context_obj.prefix, new BlockchainContextMapping(_DSUI_Context_obj)],
  [_MSUI_context_obj.prefix, new BlockchainContextMapping(_MSUI_context_obj)],
]);

export const all_contexts = readable<Map<string, IBlockchainContextMapping>>(_contexts_map);

class UISelectedContext implements Writable<string> {
  // **WATCH OUT** must match context _default_context_map above.
  private _store_obj = writable<string>(LSUI_CONTEXT_KEY);

  subscribe(listener: Subscriber<string>): Unsubscriber {
    const _unsubscriber = this._store_obj.subscribe(listener);
    return () => {
      _unsubscriber();
    };
  }

  set(value: string) {
    if (get(this._store_obj) != value) {
      this._store_obj.set(value);
    }
  }

  update(updater: Updater<string>): void {
    return this._store_obj.update(updater);
  }
}

export const createBoundedUISelectedContext = (): Writable<string> => {
  const the_store = new UISelectedContext();

  return {
    subscribe: the_store.subscribe.bind(the_store),
    set: the_store.set.bind(the_store),
    update: the_store.subscribe.bind(the_store),
  };
};

// Need to do binding because can be used from "this: void" callers.
export const ui_selected_context = createBoundedUISelectedContext();

export function context_key_to_obj(key: string): IBlockchainContext {
  const selected_context = _contexts_map.get(key);
  if (selected_context == undefined) {
    return _default_context_map.context; // TODO Report bug to sentry.
  }
  return selected_context.context;
}

export function context_key_to_stores(key: string): IBlockchainStores {
  const selected_context = _contexts_map.get(key);
  if (selected_context == undefined) {
    // TODO report sentry error.
    return _default_context_map.stores;
  }
  return selected_context.stores;
}

// Register the callback done from StateLoop.
/*
const _context_freeze = new WriterReadersMutex();

let _isFirstLoopIteration = true;
let _prev_context_key = "";

StateLoop.get_instance().loop_callback_get_context = 
(): IBlockchainContext => {
        const key = get(ui_selected_context);
        if (_isFirstLoopIteration) {
            _prev_context_key = key;
            _isFirstLoopIteration = false;
        } else if( key != _prev_context_key ) {
            _context_freeze.writer_acquire();
            // TODO Reset all reactive contexts.
            _context_freeze.writer_release();
        }
        return context_key_to_obj(key);
    }
    */

// https://en.wikipedia.org/wiki/Readers%E2%80%93writer_lock

// Reactive Abstraction
abstract class ReactiveAbstraction<T extends IContextKeyed> extends LoadStore<T, string> {
  abstract get_store_from_context(context: IBlockchainStores): Readable<T>;

  private _current_context: IBlockchainContext = _default_context_map.context;
  private _selected_context_obj?: Readable<T>;
  private _unsubscriber_context_obj?: Unsubscriber;

  protected onRun(id: number, new_dependency?: string): void {
    // Switch the subscription when the context changes.
    if (new_dependency && this._current_context.prefix !== new_dependency) {
      this._current_context = context_key_to_obj(new_dependency);
      this.subscribe_selected();
    }
    // Always succeed (no fetch/promise that can fail done here).
    this.report_in_sync(id, true);
  }

  private subscribe_selected(): void {
    this.unsubscribe_selected();
    const stores = context_key_to_stores(this._current_context.prefix);
    this._selected_context_obj = this.get_store_from_context(stores);
    this._unsubscriber_context_obj = this._selected_context_obj?.subscribe((value: T) => {
      // This is called whenever the selected context object changes.

      // Make sure the value is related to the selected context.
      // (this might be an unnecessary check, but added here for safety).
      if (value && value.context_key !== this._current_context.prefix) {
        // TODO Sentry log to see if really never happening.
        return;
      }

      //console.log("context value="+(value?value.toString():"undefined"));
      this.set(value);
    });
  }

  private unsubscribe_selected(): void {
    this._unsubscriber_context_obj?.();
    this._unsubscriber_context_obj = undefined;
    this._selected_context_obj = undefined;
  }

  protected onFirstSubscribe(): void {
    this.subscribe_selected();
  }
  protected onLastUnsubscribe(): void {
    this.unsubscribe_selected();
  }

  public constructor() {
    const retrig = 0;
    super(retrig, ui_selected_context);
  }
}

class RA_EpochLatest extends ReactiveAbstraction<EpochLatest> {
  get_store_from_context(stores: IBlockchainStores): Readable<EpochLatest> {
    //const v = JSON.stringify(stores.epoch_stores); // .epoch_latest
    //console.log("get_store_from_context called ="+(v?v.toString():"undefined"));
    return stores.epoch_stores.epoch_latest;
  }
}

class RA_EpochLeaderboard extends ReactiveAbstraction<EpochLeaderboard> {
  get_store_from_context(stores: IBlockchainStores): Readable<EpochLeaderboard> {
    //console.log("get_store_from_context called");
    return stores.epoch_stores.epoch_leaderboard;
  }
}

class RA_EpochLeaderboardHeader extends ReactiveAbstraction<IEpochETA> {
  get_store_from_context(stores: IBlockchainStores): Readable<IEpochETA> {
    //console.log("get_store_from_context called");
    return stores.epoch_stores.epoch_leaderboard_header;
  }
}

class RA_EpochValidators extends ReactiveAbstraction<EpochValidators> {
  get_store_from_context(stores: IBlockchainStores): Readable<EpochValidators> {
    //console.log("get_store_from_context called");
    return stores.epoch_stores.epoch_validators;
  }
}

class RA_EpochValidatorsHeader extends ReactiveAbstraction<IEpochETA> {
  get_store_from_context(stores: IBlockchainStores): Readable<IEpochETA> {
    //console.log("get_store_from_context called");
    return stores.epoch_stores.epoch_validators_header;
  }
}

// TODO Change to readable...
// When subscribing, subscribe to the sub-object.
// Sub-object should do nothing when not subscribed.
// When context change, unsubscribe/subscribe sub-object.
// Example of use:
//     $epoch_latest reacts whenever the EpochLatest value or context changes.
//     epoch_latest_abstraction.set_context_data() to change data for one of the context.

export const epoch_latest_abstraction = new RA_EpochLatest(); // For logic to store and control the abstracted value.
export const epoch_latest = epoch_latest_abstraction as Readable<EpochLatest>; // For UI or derived stores.

// Leaderboard abstractions.
export const epoch_leaderboard_abstraction = new RA_EpochLeaderboard(); // For logic to store and control the abstracted value.
export const epoch_leaderboard = epoch_leaderboard_abstraction as Readable<EpochLeaderboard>; // For UI or derived stores.

export const epoch_leaderboard_header_abstraction = new RA_EpochLeaderboardHeader(); // For logic to store and control the abstracted value.
export const epoch_leaderboard_header = epoch_leaderboard_header_abstraction as Readable<IEpochETA>; // For UI or derived stores.

// Validators abstractions.
export const epoch_validators_abstraction = new RA_EpochValidators(); // For logic to store and control the abstracted value.
export const epoch_validators = epoch_validators_abstraction as Readable<EpochValidators>; // For UI or derived stores.

export const epoch_validators_header_abstraction = new RA_EpochValidatorsHeader(); // For logic to store and control the abstracted value.
export const epoch_validators_header = epoch_validators_header_abstraction as Readable<IEpochETA>; // For UI or derived stores.
