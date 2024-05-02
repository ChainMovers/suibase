// JSON storage of all data (config, state and status) related to a Suibase installation.
//
// Multiple instance are expected for a given data for doing comparison and detect deltas.
//
// That object is responsible only to store the data.
//
// The actual mean to get the data is outside this object.
//
export class SuibaseGlobalStates {
  public loaded = false;

  public uiSelectedContext = "DSUI";
  public uiSelectedContextCallback: (newUiSelectedContext: string) => void = (
    // eslint-disable-next-line
    _newUiSelectedContext: string
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  ) => {};

  public serialize(): string {
    return JSON.stringify(this, (key, value) => {
      // Do not serialize callbacks.
      if (key === "uiSelectedContextCallback") {
        return undefined;
      }
      return value;
    });
  }

  public deserialize(json: string) {
    const newData = JSON.parse(json);
    // Remember which callback will need to be performed.
    let doUiSelectedContextCallback = false;
    if (newData.uiSelectedContextCallback !== this.uiSelectedContextCallback) {
      doUiSelectedContextCallback = true;
    }

    // Update this object with latest.
    Object.assign(this, newData);

    // Do the callbacks.
    if (doUiSelectedContextCallback) {
      this.uiSelectedContextCallback(this.uiSelectedContext);
    }
  }
}

export class SuibaseData {
  // SuibaseData is a singleton.
  private static instance?: SuibaseData;
  public static getInstance(): SuibaseData {
    if (!SuibaseData.instance) {
      SuibaseData.instance = new SuibaseData();
    }
    return SuibaseData.instance;
  }

  // Global variables.
  public globalStates: SuibaseGlobalStates;

  private constructor() {
    this.globalStates = new SuibaseGlobalStates();
  }

  public static activate() {
    console.log("SuibaseData.activateForExtension() called");
    if (SuibaseData.instance) {
      console.log("Error: SuibaseData.activate() called more than once");
      return;
    }

    SuibaseData.getInstance(); // Create the singleton
  }

  public static deactivate() {
    if (SuibaseData.instance) {
      delete SuibaseData.instance;
      SuibaseData.instance = undefined;
    } else {
      console.log("Error: SuibaseData.deactivate() called out of order");
    }
  }
}
