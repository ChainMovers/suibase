// General purpose custom svelte stores.
//
//     https://www.stevekinney.net/writing/svelte-stores
//     https://monad.fi/en/blog/svelte-custom-stores/
//
import { writable, type Subscriber, type Writable, type Readable, type Unsubscriber } from "svelte/store";
import { Mutex } from "async-mutex";

/*
// A Readable<number> that increments once per minute.
//
export class OneMinuteTickStore implements Readable<number> {
    private static _instance: OneMinuteTickStore;

    private readonly _store_obj: Writable<number>;
    private _loop_mutex: Mutex = new Mutex();

    // Private constructor forces use of get_instance instead.
    private constructor() {
        this._store_obj = writable<number>(0);
    }

    public static get_instance(): OneMinuteTickStore {
        if (!OneMinuteTickStore._instance) {
            OneMinuteTickStore._instance = new OneMinuteTickStore();
            if (browser) {
                // Initiate periodic calls (once per minute).
                setTimeout(() => {
                    OneMinuteTickStore._instance._sync_run();
                }, 6000);
            }
        }

        return OneMinuteTickStore._instance;
    }

    private _sync_run(): void {
        // Use to call _async_run() when the caller does
        // not care for the returned promise or error.
        //
        // This also eliminate eslint warning when the returned promise is unused.
        this._async_run().catch((err) => console.log(err));
    }

    private async _async_run(): Promise<void> {
        await this._loop_mutex.runExclusive(() => {
            this._store_obj.update((n: number) => {
                return n + 1;
            });

            // Retrig a call in one minute.
            setTimeout(() => {
                this._sync_run();
            }, 6000);
        });
    }

    public subscribe(listener: Subscriber<number>) {
        return () => {
            return this._store_obj.subscribe(listener);
        };
    }
}

// Create the OneMinuteTickStore singleton.
export const one_minute_tick = OneMinuteTickStore.get_instance();
*/

// Store with following functionality:
//   - Never "blocking" on change from the dependency.
//   - Can optionally run periodically while there
//     is at least one subscriber.
//   - All onRun calls are exclusive (mutex protected)

// T: The type stored by this LoadStore
// D: The type stored by the Readable<D> dependency.
export abstract class LoadStore<T, D> implements Readable<T> {
  protected readonly _inner: Writable<T>;
  protected readonly _dependency: Readable<D>;
  private readonly _retrig_default: number; // max targeted milliseconds delay between retrig (0 for never retrig).
  private readonly _always_retrig: boolean;
  private _unsubscriber_dependency: Unsubscriber | undefined;

  private _subscriber_count = 0;
  private _loop_mutex: Mutex = new Mutex();

  protected abstract onRun(id: number, new_dependency_value?: D): void;

  protected abstract onFirstSubscribe(): void;
  protected abstract onLastUnsubscribe(): void;

  private _sync_run(id: number, new_dependency?: D): void {
    // Use to call _async_run() when the caller does
    // not care for the returned promise or error.
    // (e.g. setTimeout).
    //
    // This also eliminate eslint warning when the returned promise is unused.
    this._async_run(id, new_dependency).catch((err) => console.log(err));
  }

  private async _async_run(id: number, new_dependency?: D): Promise<void> {
    // Not sure if mutex is needed, but play safe here because this class
    // might later get involve in a mix of async/sync complexities.
    await this._loop_mutex.runExclusive(() => {
      //console.log( "_async_run new_dependency="+(new_dependency?new_dependency.toString():"undefined"));
      this.onRun(id, new_dependency);
    });
  }

  // Retrig and back-off logic.
  private _enabled_retrig: boolean;
  private _in_failure: boolean;
  private _tx_id: number;
  private _tx_id_for_retrig: number;
  private _effective_delay: number;

  private retrig_onRun(): void {
    this._tx_id++;
    if (this._enabled_retrig) {
      this._tx_id_for_retrig = this._tx_id;
      setTimeout(
        (id) => {
          this._sync_run(id, undefined);
        },
        this._effective_delay,
        this._tx_id_for_retrig
      );
    }
  }

  private retrig_onRun_now(): void {
    this._tx_id++;
    if (this._enabled_retrig) {
      this._tx_id_for_retrig = this._tx_id;
      setTimeout(
        (id) => {
          this._sync_run(id, undefined);
        },
        0,
        this._tx_id_for_retrig
      );
    }
  }

  private force_onRun(new_dependency: D): void {
    this._tx_id++;
    setTimeout(
      (id, v) => {
        this._sync_run(id, v);
      },
      0,
      this._tx_id,
      new_dependency
    );
  }

  // Every onRun *must* call either report_in_sync() or report _done().
  //
  // report_in_sync() is for a store that does retry of loading an external
  // source with backoff delay when no success (not in-synch).
  //
  // report_done() is for a store that wants to be periodically called
  // regardless of being in synch or not with an external source.
  //
  // It is OK to do one of these call a lot later in a promise.
  protected report_done(id: number) {
    if (!this._retrig_default) return; // Don't care.
    if (id == this._tx_id_for_retrig) {
      this.retrig_onRun();
    }
  }

  protected report_in_sync(id: number, in_sync: boolean): void {
    if (!this._retrig_default) return; // Don't care.
    if (this._always_retrig) {
      // Caller should call report_done() directly instead, but
      // lets save the day by doing the right thing here.
      return;
    }
    const is_the_retrig = id == this._tx_id_for_retrig;
    let call_retrig = false;

    if (in_sync) {
      // Logic for any onRun observing the data to be in-sync.
      if (this._in_failure) {
        this._in_failure = false;
        this._effective_delay = this._retrig_default;
      }
      call_retrig = is_the_retrig;
    } else if (!this._in_failure) {
      // Not in-sync for first time, adjust for quick retrig
      // (previous retrig will be noop unless it succeeds).
      this._in_failure = true;
      this._effective_delay = 50; // 50 millisecond (will backoff on further retry)
      call_retrig = true;
    } else if (is_the_retrig) {
      // Not in-sync and this is the second "retry".
      // Slowly back-off.
      let new_delay = this._effective_delay + 1000;
      if (new_delay > this._retrig_default) new_delay = this._retrig_default;
      this._effective_delay = new_delay;
      call_retrig = true;
    }

    if (call_retrig) {
      this.retrig_onRun();
    }
  }

  protected constructor(retrig: number, dependency: Readable<D>, always_retrig = false) {
    this._dependency = dependency;
    this._retrig_default = retrig;
    this._effective_delay = retrig;
    this._in_failure = false;
    this._tx_id = 1;
    this._tx_id_for_retrig = 0;
    this._enabled_retrig = false;
    this._always_retrig = always_retrig;
    this._inner = writable<T>(undefined);
  }

  public subscribe(listener: Subscriber<T>): Unsubscriber {
    if (this._subscriber_count == 0) {
      this._enabled_retrig = true;

      // Derived does its own logic.
      this.onFirstSubscribe();

      // Initiate an immediate retrig if last known state was in-error or retrig
      // expected to always happen.
      if (this._in_failure || this._always_retrig) {
        this.retrig_onRun_now();
      }

      // Subscription to dependency.
      this._unsubscriber_dependency = this._dependency?.subscribe((value: D) => {
        // This is called whenever the dependency is initialized or changed.
        // For Reactive Abstraction, that means whenever the context changes.
        // Note: This always gets called immediately on subscription.
        this.force_onRun(value);
      });
    }

    this._subscriber_count += 1;
    const _unsubscriber = this._inner.subscribe(listener);
    return () => {
      this._subscriber_count -= 1;
      if (this._subscriber_count == 0) {
        this._enabled_retrig = false;

        if (this._unsubscriber_dependency) {
          this._unsubscriber_dependency();
          this._unsubscriber_dependency = undefined;
        }
        // Derived does its own logic.
        this.onLastUnsubscribe();
      }
      _unsubscriber();
    };
  }

  protected set(value: T) {
    this._inner.set(value);
  }

  public value(): T | undefined {
    return this.value();
  }

  public get subscriber_count(): number {
    return this._subscriber_count;
  }
}
