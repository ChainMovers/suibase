import { writable, get, type Readable, type Writable, type Unsubscriber } from "svelte/store";

import dayjs from "dayjs";

import relativeTime from "dayjs/plugin/relativeTime.js";
import utc from "dayjs/plugin/utc.js";

// Layer 1
import type {
  IContextKeyed,
  IEpochRevision,
  ILoadedState,
  IEpochETA,
  IEndOfEpochFields,
} from "../L1/poc-interfaces";
import { EpochLatest, EpochLeaderboard, EpochValidators } from "../L1/json-constructed";

import { LoadStore } from "../L1/stores";

// Layer 2
import { min_headers_key, min_headers_value, global_url_proxy } from "../L2/globals";
import type { IBlockchainContext, IEpochStores } from "../L2/interfaces";
import { sleep } from "../../utils";

dayjs.extend(relativeTime);
dayjs.extend(utc);

class EpochRevision implements IEpochRevision {
  private _e: number;
  private _r: number;
  private _s: string; // Store, but ignore in comparisons.

  public get e() {
    return this._e;
  }
  public get r() {
    return this._r;
  }
  public get s() {
    return this._s;
  }

  constructor(p_e: number, p_r: number, p_s: string) {
    this._e = p_e;
    this._r = p_r;
    this._s = p_s;
  }

  public is_older_revision_of(other?: IEpochRevision): boolean {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not newer.
    }

    // Both are defined, so compare their epoch and revision.
    return other.e > this.e || (other.e === this.e && other.r > this.r);
  }

  public is_same_revision_as(other?: IEpochRevision) {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not the same.
    }
    // Both are defined, so compare their epoch and revision.
    return this.e === other.e && this.r === other.r;
  }

  public set(other?: IEpochRevision) {
    if (other === undefined) {
      this._e = 0;
      this._r = 0;
      this._s = "";
    } else {
      this._e = other.e;
      this._r = other.r;
      this._s = other.s;
    }
  }
}

// LoadStore with dependencies on EV changes.
//
// (Dependency on a Readable<EpochLatest).
//
// **********************
// API loading functions.
//
// Must:
//    - Throw an error on any failure, including failed validation.
//    - Transform response into a typed JS object and return it.
//
// **********************

abstract class EpochLoadStore<T extends IContextKeyed & IEpochRevision> extends LoadStore<T, EpochLatest> {
  private readonly _revision_needed: EpochRevision;
  private readonly _revision_loaded: EpochRevision;
  protected readonly _context: IBlockchainContext;

  public constructor(context: IBlockchainContext, dependency: Readable<EpochLatest>) {
    const retrig = 60000; // 1 minute.
    super(retrig, dependency);
    this._revision_needed = new EpochRevision(0, 0, "");
    this._revision_loaded = new EpochRevision(0, 0, "");
    this._context = context;
  }

  // Load "StaticDynamic" data which is specific to an epoch.
  protected async api_load_sd(epoch_revision: EpochRevision, sd_name: string): Promise<T> {
    // force-cache will force browser to use the cache even if expired or stale.
    // This is what we want since the response here is immutable for an 'epoch_latest.s'
    const url_path = `${this._context.server}/sd/${sd_name}`;
    const response = await fetch(
      `${get(global_url_proxy)}/${url_path}/${epoch_revision.e}/${epoch_revision.s}`,
      { cache: "force-cache" }
    );
    const values = (await response.json()) as T;
    return Promise.resolve(values);
  }

  protected abstract api_load(context: IBlockchainContext, epoch_revision: EpochRevision): Promise<T>;

  protected onRun(id: number, new_dependency?: EpochLatest): void {
    // Is the new_dependency better than the one already loaded/loading?
    // If yes, then use it.
    if (new_dependency) {
      if (this._revision_needed.is_older_revision_of(new_dependency))
        this._revision_needed.set(new_dependency);
    }

    // Need to try loading?
    if (this._revision_loaded.is_older_revision_of(this._revision_needed)) {
      // Call a sync load API. Assume caller will do it ASYNC.
      this.api_load(this._context, this._revision_needed)
        .then((new_leaderboard) => {
          //console.log( "leaderboard="+JSON.stringify(new_leaderboard));
          if (this._revision_needed.is_same_revision_as(new_leaderboard)) {
            this._revision_loaded.set(this._revision_needed);
            this.set_store(new_leaderboard);
            this.report_in_sync(id, true);
          } else {
            this.report_in_sync(id, false);
          }
        })
        .catch((err) => {
          console.log(err);
          this.report_in_sync(id, false);
        });
    }
  }

  protected abstract set_store(new_value: T): void;

  protected onFirstSubscribe(): void {
    // eslint-disable @typescript-eslint/no-empty-function
    //console.log( "firstSubscribe" );
  }

  protected onLastUnsubscribe(): void {
    // eslint-disable @typescript-eslint/no-empty-function
    //console.log( "lastUnsubscribe" );
  }
}

class EpochLoadStoreLeaderboard extends EpochLoadStore<EpochLeaderboard> {
  // TODO: Try to move all this to base class.
  private async async_api_load(
    context: IBlockchainContext,
    epoch_revision: EpochRevision
  ): Promise<EpochLeaderboard> {
    const resp_json: unknown = await this.api_load_sd(epoch_revision, "TOPV");
    //console.log( "async_api_load of leaderboard="+JSON.stringify(resp_json));
    const values: EpochLeaderboard = new EpochLeaderboard(resp_json, context.prefix);
    return Promise.resolve(values);
  }

  protected api_load(context: IBlockchainContext, epoch_revision: EpochRevision): Promise<EpochLeaderboard> {
    return this.async_api_load(context, epoch_revision); //.then().catch((err)=>{ console.log(err);});
  }

  set_store(new_value: EpochLeaderboard): void {
    this.set(new_value);
  }
}

class EpochLoadStoreValidators extends EpochLoadStore<EpochValidators> {
  // TODO: Try to move all this to base class, except for ALLV
  private async async_api_load(
    context: IBlockchainContext,
    epoch_revision: EpochRevision
  ): Promise<EpochValidators> {
    const resp_json: unknown = await this.api_load_sd(epoch_revision, "ALLV");
    //console.log( "async_api_load of leaderboard="+JSON.stringify(resp_json));
    const values: EpochValidators = new EpochValidators(resp_json, context.prefix);
    return Promise.resolve(values);
  }

  protected api_load(context: IBlockchainContext, epoch_revision: EpochRevision): Promise<EpochValidators> {
    return this.async_api_load(context, epoch_revision); //.then().catch((err)=>{ console.log(err);});
  }

  set_store(new_value: EpochValidators): void {
    this.set(new_value);
  }
}

// Store for ETA of next update for another store.
//
// Functionality:
//   - Never "blocking" on change from the dependencies.
//   - Updates once per minute while there is at least one subscriber.
//   - Output includes a derived ILoadedState that considers
//     matching epoch versioning from the dependencies.

// Type used for the time source dependency.
type IEpochTimeSource = IEndOfEpochFields & ILoadedState;

// Stored value returned by ETAStore.
class ETAValue implements IEpochETA {
  isLoaded: boolean;
  e: number; // Epoch number.
  remaining: string; // Time to end of epoch.
  elapsed: string; // Time since beginning of epoch.
  tick: number;
  context_key: string;

  constructor(context: string) {
    this.isLoaded = false;
    this.e = 0;
    this.remaining = "";
    this.elapsed = "";
    this.tick = 0;
    this.context_key = context;
  }
}

// Class used for some internal variables of ETASTore only.
class EpochTimeSource implements IEndOfEpochFields {
  private _e: number;
  private _t: number;
  private _y: number;

  public get e() {
    return this._e;
  }
  public get t() {
    return this._t;
  }
  public get y() {
    return this._y;
  }

  constructor(p_e: number, p_t: number, p_y: number) {
    this._e = p_e;
    this._t = p_t;
    this._y = p_y;
  }

  public set(other?: IEndOfEpochFields) {
    if (other === undefined) {
      this._e = 0;
      this._t = 0;
      this._y = 0;
    } else {
      this._e = other.e;
      this._t = other.t;
      this._y = other.y;
    }
  }
}

export class ETAStore<D extends ILoadedState & IEpochRevision> extends LoadStore<IEpochETA, D> {
  private readonly _time_source_fields: EpochTimeSource;
  private readonly _dependency_fields: EpochRevision;
  private readonly _time_source: Readable<IEpochTimeSource>;
  private readonly _eta_value: ETAValue;
  private _unsubscriber_time_source: Unsubscriber | undefined;
  protected readonly _context: IBlockchainContext;

  public constructor(
    context: IBlockchainContext,
    dependency: Readable<D>,
    time_source: Readable<IEpochTimeSource>
  ) {
    const retrig = 60000; // 1 minute.
    const always_retrig = true;
    super(retrig, dependency, always_retrig);
    this._time_source = time_source;
    this._context = context;
    this._eta_value = new ETAValue(this._context.prefix);
    this._time_source_fields = new EpochTimeSource(0, 0, 0);
    this._dependency_fields = new EpochRevision(0, 0, "");
  }

  private update_store(): void {
    // isLoaded == true only when:
    //    - dependency and time_source epoch are matching.
    //    - dependency is defined (epoch != 0)
    let isLoadedChanged = false;
    let isAnyFieldChanged = false;

    const new_e = this._time_source_fields.e;
    if (this._dependency_fields.e != 0 && this._dependency_fields.e == new_e) {
      if (this._eta_value.e != new_e) {
        this._eta_value.e = new_e;
        isAnyFieldChanged = true;
      }

      const current = dayjs().unix(); // Now
      const djs_start = dayjs.unix(this._time_source_fields.t);
      const new_elapsed = djs_start.fromNow();
      if (this._eta_value.elapsed != new_elapsed) {
        this._eta_value.elapsed = new_elapsed;
        isAnyFieldChanged = true;
      }

      const e_latest = this._time_source_fields.y;
      let new_remaining = "";
      if (e_latest != 0) {
        if (current >= e_latest) {
          new_remaining = "in progress";
        } else {
          const djs_end = dayjs.unix(e_latest);
          new_remaining = djs_end.fromNow();
        }
      }
      if (this._eta_value.remaining != new_remaining) {
        this._eta_value.remaining = new_remaining;
        isAnyFieldChanged = true;
      }

      if (!this._eta_value.isLoaded) {
        this._eta_value.isLoaded = true;
        isLoadedChanged = true;
      }
    } else {
      if (this._eta_value.isLoaded) {
        this._eta_value.isLoaded = false;
        this._eta_value.remaining = "";
        this._eta_value.elapsed = "";
        isLoadedChanged = true;
      }
    }

    if (isLoadedChanged || isAnyFieldChanged) {
      this._eta_value.tick++;
      this.set(this._eta_value);
    }
  }

  protected onRun(id: number, new_dependency?: D): void {
    if (new_dependency) {
      if (new_dependency.isLoaded) {
        this._dependency_fields.set(new_dependency);
      } else {
        this._dependency_fields.set(undefined);
      }
    }
    this.update_store();
    this.report_done(id);
  }

  protected onFirstSubscribe(): void {
    // Subscribe to the time_source.
    this._unsubscriber_time_source = this._time_source?.subscribe(
      (new_timesource_value: IEpochTimeSource) => {
        // This is called whenever the time_source is initialized or changed.
        // Note: This always gets called immediately on subscription.
        if (new_timesource_value?.isLoaded) {
          this._time_source_fields.set(new_timesource_value);
        } else {
          this._time_source_fields.set(undefined);
        }
        this.update_store();
      }
    );
  }

  protected onLastUnsubscribe(): void {
    // Unsubscribe from the time_source (as needed).
    this._unsubscriber_time_source?.();
    this._unsubscriber_time_source = undefined;
  }
}

export class EpochStores implements IEpochStores {
  // Public read-only facades.
  public readonly epoch_latest: Readable<EpochLatest>;
  public readonly epoch_leaderboard: Readable<EpochLeaderboard>;
  public readonly epoch_leaderboard_header: Readable<IEpochETA>;
  public readonly epoch_validators: Readable<EpochValidators>;
  public readonly epoch_validators_header: Readable<IEpochETA>;

  // Fetch parameters.
  private _fetch_headers_ev_sd = new Headers();
  private _optional_fields = {};

  // Private R/W stores.
  private readonly _epoch_latest_instance: Writable<EpochLatest>;
  private readonly _epoch_leaderboard_instance: EpochLoadStoreLeaderboard;
  private readonly _epoch_leaderboard_header_instance: ETAStore<EpochLeaderboard>;
  private readonly _epoch_validators_instance: EpochLoadStoreValidators;
  private readonly _epoch_validators_header_instance: ETAStore<EpochValidators>;

  // Context will never change for an instance of EpochStores.
  private _context: IBlockchainContext;
  constructor(context: IBlockchainContext) {
    this._context = context;
    this._fetch_headers_ev_sd.append(min_headers_key(), min_headers_value());
    this._optional_fields = { headers: this._fetch_headers_ev_sd, mode: "cors" };

    this._epoch_latest_instance = writable<EpochLatest>(undefined);
    this.epoch_latest = this._epoch_latest_instance as Readable<EpochLatest>;

    // Leaderboard related stores.
    this._epoch_leaderboard_instance = new EpochLoadStoreLeaderboard(this._context, this.epoch_latest);
    this.epoch_leaderboard = this._epoch_leaderboard_instance as Readable<EpochLeaderboard>;

    this._epoch_leaderboard_header_instance = new ETAStore<EpochLeaderboard>(
      this._context,
      this.epoch_leaderboard,
      this.epoch_latest
    );
    this.epoch_leaderboard_header = this._epoch_leaderboard_header_instance as Readable<IEpochETA>;

    // Validators related stores.
    this._epoch_validators_instance = new EpochLoadStoreValidators(this._context, this.epoch_latest);
    this.epoch_validators = this._epoch_validators_instance as Readable<EpochValidators>;

    this._epoch_validators_header_instance = new ETAStore<EpochValidators>(
      this._context,
      this.epoch_validators,
      this.epoch_latest
    );
    this.epoch_validators_header = this._epoch_validators_header_instance as Readable<IEpochETA>;
  }

  private _is_first_update_call = true;
  private _time_since_last_call = 0;

  public async update_ev(force_refresh: boolean): Promise<void> {
    if (this._is_first_update_call) {
      this._is_first_update_call = false;
      force_refresh = true;
    }

    const now = dayjs().unix();
    if (!force_refresh && this._time_since_last_call) {
      const diff_secs = now - this._time_since_last_call;
      if (diff_secs < 15)
        // Time between ev load (ignored on force).
        return;
    }
    this._time_since_last_call = now;

    const new_epoch_latest = await this.api_load_ev();

    if (new_epoch_latest) {
      const cur_epoch_latest: EpochLatest = get(this._epoch_latest_instance);
      if (!cur_epoch_latest) {
        this._epoch_latest_instance.set(new_epoch_latest);
      } else if (!cur_epoch_latest.isEquivalent(new_epoch_latest)) {
        // Use what was received only if version and/or revision are moving forward...
        if (
          new_epoch_latest.e > cur_epoch_latest.e ||
          (new_epoch_latest.e == cur_epoch_latest.e && new_epoch_latest.r > cur_epoch_latest.r)
        ) {
          // Set store about latest epoch versioning information.
          // This may trig further data load+update for all derived
          // data stores.
          this._epoch_latest_instance.set(new_epoch_latest);
        }
      }
    }
  }

  private static is_same_revision(a: IEpochRevision, b: IEpochRevision) {
    if (a === undefined && b === undefined) {
      return true; // Both undefined arbitrarily default to "same".
    }
    if (a === undefined || b === undefined) {
      return false; // Only one of the two undefined are necessarily "different"
    }
    // Both are defined, so compare their epoch and revision.
    return a.e == b.e && a.r == b.r;
  }

  private async api_load_ev(): Promise<EpochLatest> {
    const url_path = `${this._context.server}/ev`;
    const response = await fetch(`${get(global_url_proxy)}/${url_path}`, this._optional_fields);
    if (!response.ok) {
      const message = `An error has occurred: ${response.status} for ${url_path}`;
      throw new Error(message);
    }
    const resp_json: unknown = await response.json();
    const values: EpochLatest = new EpochLatest(resp_json, this._context.prefix);
    return Promise.resolve(values);
  }
}
