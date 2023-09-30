import { SuibaseJSONStorage, SuibaseJson } from "../common/SuibaseJSONStorage";

// Readonly interface of a suibaseJSONStorage singleton.
//
// This wrapper handles also all update messages FROM the extension and
// allow for any components to set callback into it.
export class GlobalStorage {
  private static instance?: GlobalStorage;
  private suibaseJSONStorage?: SuibaseJSONStorage;

  // Fit the VSCode initialization pattern.
  // Constructor should be called only from GlobalStorage.activate().
  // Release of resources done by GlobalStorage.deactivate().
  private constructor() {
    this.suibaseJSONStorage = new SuibaseJSONStorage();
  }

  public static activate() {
    if (!GlobalStorage.instance) {
      GlobalStorage.instance = new GlobalStorage();
    }
    return GlobalStorage.instance;
  }

  public static deactivate() {
    let instance = GlobalStorage.instance;
    if (instance) {
      delete instance.suibaseJSONStorage;
    }
    delete GlobalStorage.instance;
    GlobalStorage.instance = undefined;
  }

  public static getInstance(): GlobalStorage | undefined {
    if (!GlobalStorage.instance) {
      console.log("Error: GlobalStorage.getInstance() called before activate()");
    }
    return GlobalStorage.instance;
  }
}
