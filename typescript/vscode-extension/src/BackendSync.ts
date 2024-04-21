import { SuibaseJSONStorage } from "./common/SuibaseJSONStorage";
import { API_URL, WORKDIRS_KEYS } from "./common/Consts";
import { Mutex } from "async-mutex";
import { BaseWebview } from "./bases/BaseWebview";
import { UpdateVersions } from "./common/ViewMessages";

// Readonly interface of a suibaseJSONStorage singleton.
//
// This wrapper handles also all update messages FROM the extension and
// allow for any components to set callback into it.
export class BackendSync {
  private static sInstance?: BackendSync;
  private mSuibaseJSONStorage?: SuibaseJSONStorage;

  private mWorkdir: string; // One of "localnet", "mainnet", "testnet", "devnet".

  //private mCurMethodUUID: string;
  //private mCurDataUUID: string;

  // Fit the VSCode initialization pattern.
  // Constructor should be called only from GlobalStorage.activate().
  // Release of resources done by GlobalStorage.deactivate().
  private constructor() {
    //this.mCurMethodUUID = "";
    //this.mCurDataUUID = "";
    this.mWorkdir = "";
    this.mSuibaseJSONStorage = new SuibaseJSONStorage();
  }

  public static activate() {
    if (!BackendSync.sInstance) {
      BackendSync.getInstance();
    } else {
      console.error("GlobalStorage.activate() called more than once");
    }
  }

  public static deactivate() {
    const instance = BackendSync.sInstance;
    if (instance) {
      delete instance.mSuibaseJSONStorage;
    }
    delete BackendSync.sInstance;
    BackendSync.sInstance = undefined;
  }

  public static getInstance(): BackendSync {
    if (!BackendSync.sInstance) {
      BackendSync.sInstance = new BackendSync();
      setTimeout(() => {
        if (BackendSync.sInstance) {
          BackendSync.sInstance.syncLoop();
        }
      }, 1);
    }
    return BackendSync.sInstance;
  }

  public get workdir(): string {
    return this.mWorkdir;
  }

  // Allow to trig a force refresh of states handled by this loop.
  // Can be called safely from anywhere.
  public forceLoopRefresh(): void {
    this.syncLoop(true);
  }

  private loopMutex: Mutex = new Mutex();

  private syncLoop(forceRefresh = false): void {
    //console.log("StateLoop::_sync_loop() force_refresh=" + force_refresh);
    // Used for calling the loop() async function when the caller does
    // not care for the returned promise or error.
    //
    // This also eliminate eslint warning when the returned promise is unused.
    this.asyncLoop(forceRefresh).catch((err) => console.log(err));
  }

  private async asyncLoop(forceRefresh: boolean): Promise<void> {
    await this.loopMutex.runExclusive(async () => {
      this.update();

      if (forceRefresh === false) {
        // Schedule another call in one second.
        setTimeout(() => {
          this.syncLoop(forceRefresh);
        }, 1000);
      }
    }); // End of loop_mutex section
  }

  private async fetchBackend<T = any>(method: string, workdir: string): Promise<T> {
    // Do a POST request equivalent to:
    //   curl -H "Content-Type: application/json" --data '{ "id":1, "jsonrpc":"2.0", "method":"getVersions", "params": {"workdir":"localnet"}}' http://0.0.0.0:44399
    //
    // On error, throw an exception.
    // On success, return the parsed JSON response.
    const headers = {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      "Content-Type": "application/json",
    };

    const body = {
      id: 1,
      jsonrpc: "2.0",
      method: method,
      params: workdir === "" ? {} : { workdir: workdir },
    };

    try {
      const fetch = await import("node-fetch").then((fetch) => fetch.default);
      const response = await fetch(API_URL, {
        method: "POST",
        headers: headers,
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        throw new Error("getVersions fetch not ok");
      }
      let json: T = (await response.json()) as T;
      BackendSync.validateHeader(json, method);
      return json;
    } catch (error) {
      let errorMsg = `Error in fetchBackend ${method} ${workdir}: ${error}`;
      console.error(errorMsg);
      throw error;
    }
  }

  private async fetchGetVersions(workdir: string) {
    return await this.fetchBackend("getVersions", workdir);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private static validateHeader(json: any, methodExpected: string) {
    // Throw an error on any problem detected.
    if (json.jsonrpc !== "2.0") {
      throw new Error("Invalid JSON-RPC version");
    }
    if (json.result.header.method !== methodExpected) {
      throw new Error("Invalid method");
    }
  }

  public async update() {
    // Get the global states from the backend.

    // The VersionsUpdate message is periodically pushed to the views.
    //
    // The views then identify if they need to synchronize further with the extension
    // and trig update requests messages as needed.
    try {
      await this.updateUsingBackend();
    } catch (error) {
      // Do nothing, assume the caller will retry later.
    }
  }

  private async updateUsingBackend() {
    // Do getVersions for every possible workdir.
    //
    // TODO Optimize to do this to retrieve all only when dashboard is visible, otherwise, just update active.

    // Iterate the WORKDIRS_KEYS
    for (let workdirIdx = 0; workdirIdx < WORKDIRS_KEYS.length; workdirIdx++) {
      let workdir = WORKDIRS_KEYS[workdirIdx];
      const data = await this.fetchGetVersions(workdir);
      if (data) {
        try {
          // This is an example of response stored in data:
          //  {"jsonrpc":"2.0","result":{
          //   "header":{"method":"getVersions", "methodUuid":"8HIGKAE8L54850LDHQ7NN9EDG0","dataUuid":"067F4QSD45QPT1BUET42FFHM0S","key":"localnet"},
          //   "versions":[{"method":"getWorkdirStatus","methodUuid":"ET1217DP0503LF4PFMB49J0LUC","dataUuid":"067F4QSD45QPT1BUET3JOJPQ50","key":"localnet"}],
          //   "asuiSelection":"localnet"},
          //   "id":2}

          // Broadcast VersionsUpdate message to all the views.
          // The views will then decide if they need to synchronize further with the extension.
          BaseWebview.broadcastMessage(new UpdateVersions(workdirIdx, data));
        } catch (error) {
          const errorMsg = `Error in load_from_backend: ${error}. Data: ${JSON.stringify(data)}`;
          console.error(errorMsg);
          throw new Error(errorMsg);
        }
      }
    }
  }

  // Update the data for the context requested.

  // Verify if the asuiSelection match the current context, if not, then switch to it and retrieve the new context.
}
