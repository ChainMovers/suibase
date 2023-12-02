import dayjs from "dayjs";

import relativeTime from "dayjs/plugin/relativeTime.js";
import utc from "dayjs/plugin/utc.js";

// Layer 1
import { IContextKeyed, IVersioned, JSONHeader } from "../L1/poc-interfaces";
import { VersionsLatest, WorkdirStatus } from "../L1/json-constructed";

import { LoadStore } from "../L1/stores";

// Layer 2
import { min_headers_key, min_headers_value, global_url_proxy } from "../L2/globals";
import type { IBlockchainContext, IEpochStores, IGlobalsStores } from "../L2/interfaces";
import { sleep } from "../../utils";
import { writable, type Readable, type Writable, get } from "svelte/store";

dayjs.extend(relativeTime);
dayjs.extend(utc);

class UUIDRevision implements IVersioned {
  header: JSONHeader;

  public get method_uuid() {
    return this.header.methodUuid;
  }

  public get data_uuid() {
    return this.header.dataUuid;
  }

  public get method() {
    return this.header.method;
  }

  public get key() {
    return this.header.key;
  }

  constructor(p_header: JSONHeader) {
    this.header = p_header;
  }

  // The default method
  static default(): UUIDRevision {
    return new UUIDRevision(JSONHeader.default());
  }

  public is_older_revision_of(other?: IVersioned): boolean {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not newer.
    }
    return this.header.is_older_revision_of(other.header);
  }

  public is_older_header_of(other?: JSONHeader): boolean {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not newer.
    }
    return this.header.is_older_revision_of(other);
  }

  public is_same_revision_as(other?: IVersioned) {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not the same.
    }
    // Both are defined, so compare their JSONHeaders
    return other.header.methodUuid == this.header.methodUuid && other.header.dataUuid == this.header.dataUuid;
  }

  // Is this used? if not replace with set_from_JSONHeader functionality.
  public set(other?: IVersioned) {
    if (other === undefined) {
      this.header = JSONHeader.default();
    } else {
      this.header = other.header;
    }
  }

  public set_from_JSONHeader(other?: JSONHeader) {
    if (other === undefined) {
      this.header = JSONHeader.default();
    } else {
      this.header = other;
    }
  }
}

// LoadStore with dependencies on UUIDVersion changes.
//
// (Dependency on a Readable<Versions>).
//
// **********************
// API loading functions.
//
// Must:
//    - Throw an error on any failure, including failed validation.
//    - Transform response into a typed JS object and return it.
//
// **********************

abstract class UUIDLoadStore<T extends IContextKeyed & IVersioned> extends LoadStore<T, VersionsLatest> {
  private readonly _revision_needed: UUIDRevision;
  private readonly _revision_loaded: UUIDRevision;
  protected readonly _context: IBlockchainContext;

  public constructor(context: IBlockchainContext, dependency: Readable<VersionsLatest>) {
    const retrig = 60000; // 1 minute.
    super(retrig, dependency);
    this._revision_needed = UUIDRevision.default();
    this._revision_loaded = UUIDRevision.default();
    this._context = context;
  }

  // Load data from backend daemon (request for the needed_revision).
  protected async api_load_uuid(needed_revision: UUIDRevision): Promise<T> {
    // force-cache will force browser to use the cache even if expired or stale.
    // This is what we want since the response here is immutable for an 'epoch_latest.s'
    // Do a POST request equivalent to http://0.0.0.0:44399 with:
    // header is Content-Type: application/json
    // body is {"id":1,"jsonrpc":"2.0","method":"getLinks","params":{"workdir.name"}}
    const url = "http://localhost:44399"; // TODO Replace with `${this._context.server}`
    const headers = {
      "Content-Type": "application/json",
    };
    const body = {
      id: 1,
      jsonrpc: "2.0",
      method: needed_revision.method,
      params: {
        workdir: needed_revision.key,
        method_uuid: needed_revision.method_uuid,
        data_uuid: needed_revision.data_uuid,
      },
    };

    let response = await fetch(url, {
      method: "POST",
      headers: headers,
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error("Network response was not ok");
    }

    const values = (await response.json()) as T;
    return Promise.resolve(values);
  }

  protected abstract api_load(context: IBlockchainContext, uuid_revision: UUIDRevision): Promise<T>;

  protected onRun(id: number, new_dependency?: VersionsLatest): void {
    // Is the new_dependency indicate that there is a more recent version
    // then what is already loaded?
    // If yes, then try retrieving the most recent version.
    if (new_dependency && new_dependency.isLoaded) {
      const new_header = new_dependency.versions[new_dependency.version_workdir_status_idx];
      if (this._revision_needed.is_older_header_of(new_header)) {
        this._revision_needed.set_from_JSONHeader(new_header);
      }
    }

    // Need to try loading?
    if (this._revision_loaded.is_older_revision_of(this._revision_needed)) {
      console.log("onRun() trying to load new data");
      // Call a sync load API. Assume caller will do it ASYNC.
      this.api_load(this._context, this._revision_needed)
        .then((new_data) => {
          if (this._revision_needed.is_same_revision_as(new_data)) {
            this._revision_loaded.set(this._revision_needed);
            this.set_store(new_data);
            this.report_in_sync(id, true);
            console.log("onRun() loaded new_data: " + JSON.stringify(new_data));
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

class UUIDLoadStoreWorkdirStatus extends UUIDLoadStore<WorkdirStatus> {
  private async async_api_load(
    context: IBlockchainContext,
    needed_revision: UUIDRevision
  ): Promise<WorkdirStatus> {
    // Will try to load the latest revision.
    const resp_json: unknown = await this.api_load_uuid(needed_revision);
    console.log("async_api_load of WorkdirStatus=" + JSON.stringify(resp_json));
    const values: WorkdirStatus = new WorkdirStatus(resp_json, context.prefix);
    return Promise.resolve(values);
  }

  protected api_load(context: IBlockchainContext, needed_revision: UUIDRevision): Promise<WorkdirStatus> {
    return this.async_api_load(context, needed_revision); //.then().catch((err)=>{ console.log(err);});
  }

  set_store(new_value: WorkdirStatus): void {
    this.set(new_value);
  }
}

export class GlobalsStores implements IGlobalsStores {
  // Public read-only facades.
  public readonly versions_latest: Readable<VersionsLatest>;
  public readonly workdir_status: Readable<WorkdirStatus>;

  // Fetch parameters.
  /*
  private _fetch_headers_ev_sd = new Headers();
  private _optional_fields = {};
  */

  // Private R/W stores.
  private readonly _versions_latest_instance: Writable<VersionsLatest>;
  private readonly _workdir_status_instance: UUIDLoadStoreWorkdirStatus;

  // Context will never change for an instance of GlobalsStores.
  private _context: IBlockchainContext;
  constructor(context: IBlockchainContext) {
    this._context = context;

    /* this._fetch_headers_ev_sd.append(min_headers_key(), min_headers_value());
    this._optional_fields = { headers: this._fetch_headers_ev_sd, mode: "cors" };*/

    this._versions_latest_instance = writable<VersionsLatest>(undefined);
    this.versions_latest = this._versions_latest_instance as Readable<VersionsLatest>;

    // Stores that depend on versions_latest.
    this._workdir_status_instance = new UUIDLoadStoreWorkdirStatus(this._context, this.versions_latest);
    this.workdir_status = this._workdir_status_instance as Readable<WorkdirStatus>;
  }

  private _is_first_update_call = true;
  private _time_since_last_call = 0;

  public async update_versions(force_refresh: boolean): Promise<void> {
    if (this._is_first_update_call) {
      this._is_first_update_call = false;
      force_refresh = true;
    }

    const now = dayjs().unix();
    if (!force_refresh && this._time_since_last_call) {
      const diff_secs = now - this._time_since_last_call;
      if (diff_secs < 15)
        // Time between load (ignored on force).
        return;
    }
    this._time_since_last_call = now;

    const new_versions_latest = await this.api_load_versions();

    if (new_versions_latest) {
      const cur_versions_latest: VersionsLatest = get(this._versions_latest_instance);
      // console.log("update_versions() new_versions_latest=" + JSON.stringify(new_versions_latest));
      if (!cur_versions_latest) {
        this._versions_latest_instance.set(new_versions_latest);
      } else if (!cur_versions_latest.isEquivalent(new_versions_latest)) {
        // Use what was received only if version and/or revision are moving forward...
        if (cur_versions_latest.header.is_older_revision_of(new_versions_latest.header)) {
          // Set store with latest versioning information.
          // This may trig further data load+update for all derived
          // data stores.
          this._versions_latest_instance.set(new_versions_latest);
        }
      }
    }
  }
  /*
  private static is_same_revision(a: IEpochRevision, b: IEpochRevision) {
    if (a === undefined && b === undefined) {
      return true; // Both undefined arbitrarily default to "same".
    }
    if (a === undefined || b === undefined) {
      return false; // Only one of the two undefined are necessarily "different"
    }
    // Both are defined, so compare their epoch and revision.
    return a.e == b.e && a.r == b.r;
  }*/

  private async api_load_versions(): Promise<VersionsLatest> {
    const url = "http://localhost:44399"; // TODO Replace with `${this._context.server}`
    const headers = {
      "Content-Type": "application/json",
    };
    const body = {
      id: 1,
      jsonrpc: "2.0",
      method: "getVersions",
      params: {
        workdir: this._context.workdir,
      },
    };

    let response = await fetch(url, {
      method: "POST",
      headers: headers,
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error("Network response was not ok");
    }

    const resp_json = await response.json();
    const values: VersionsLatest = new VersionsLatest(resp_json, this._context.workdir, this._context.prefix);
    return Promise.resolve(values);
  }
}
