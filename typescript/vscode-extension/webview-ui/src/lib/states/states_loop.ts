// Periodic loop to update most states.
//
// This loop is controlling the safe execution order for both the
// initialization and periodical processing for most of the
// client app.
//
// Public API is done with the StateLoop::get_instance() singleton.
//
// Data Layer Design (Top has the one below):
//   --> Simple double buffering logic.
//         Initialize one state while other is the active.
//
//   $table.data    <= May change often. Any change means a new data object.
//   $table.isXXX   <= Useful abstractions.
//   $table.context <= For UI variations specific to context. Never changes.
//
//
//
// data is always json.
//   BlockchainContext
//     Consts,EpochContext,LiveContext
//
//   ReactiveAbstraction
//         data
//         isUpdating
//         isLoading
//         data_change
//         context_change
//
//       On context change:
//          unsubscribe the current data store.
//          subscribe to the new data store.
//          evaluate abstracted 'isX'
//          increment context_change.
//       On data change:
//          evaluate abstracted 'isX'
//          increment data_change.
//import { browser } from "$app/env";
import { Mutex } from "async-mutex";
import { to } from "await-to-js";

import type { IBlockchainStores } from "./L2/interfaces";
import { ui_selected_context, context_key_to_stores } from "./L4/contexts";

export class StateLoop {
  private static _instance: StateLoop;
  private _selected_context_stores?: IBlockchainStores;

  // Private constructor forces use of get_instance instead.
  private constructor() {
    // Subscribe for any context changes (at UI level).
    ui_selected_context.subscribe((selected_str: string) => {
      this._selected_context_stores = context_key_to_stores(selected_str);
      this.force_loop_refresh();
    });
  }

  public static get_instance(): StateLoop {
    if (!StateLoop._instance) {
      // Put here any code intended to be called only once
      // on "SPA initialization" (See +layout.svelte or App.svelte).
      StateLoop._instance = new StateLoop();

      /* if (browser) {*/
      // Initiate periodic calls.
      setTimeout(() => {
        StateLoop._instance._sync_loop();
      }, 1);
      /*}*/
    }

    return StateLoop._instance;
  }

  // Allow to trig a force refresh of states handled by this loop.
  // Can be called safely from anywhere.
  public force_loop_refresh(): void {
    this._sync_loop(true);
  }

  private _loop_mutex: Mutex = new Mutex();

  private _sync_loop(force_refresh = false): void {
    //console.log("StateLoop::_sync_loop() force_refresh=" + force_refresh);
    // Used for calling the loop() async function when the caller does
    // not care for the returned promise or error.
    //
    // This also eliminate eslint warning when the returned promise is unused.
    this._async_loop(force_refresh).catch((err) => console.log(err));
  }

  private async _async_loop(force_refresh: boolean): Promise<void> {
    await this._loop_mutex.runExclusive(async () => {
      // Most derived stores depends on the "globals versioning" values.
      //
      // So by loading/updating it periodically all data will eventually
      // converge to the latest globals data.
      const cur_context_stores = this._selected_context_stores;
      if (cur_context_stores && cur_context_stores.globals_stores) {
        const [err] = await to<void>(cur_context_stores.globals_stores.update_versions(force_refresh));
        if (err) {
          console.log(err);
        }
      }
      /*
      if (cur_context_stores && cur_context_stores.epoch_stores) {
        const [err] = await to<void>(cur_context_stores.epoch_stores.update_ev(force_refresh));
        if (err) {
          console.log(err);
        }
      }*/

      if (force_refresh == false) {
        // Schedule another call in one second.
        setTimeout(
          () => {
            this._sync_loop();
          },
          1000,
          force_refresh
        );
      }
    }); // End of loop_mutex section
  }
}
