// JSON storage of all data (config, state and status) related to a Suibase installation.
//
// Multiple instance are expected for a given data for doing comparison and detect deltas.
//
// When the instance is for the extension, the data is from the suibase-daemon.
// When the instance is for a view, the data is from the extension.
//
// Take note that a Svelte view further convert this data to smaller reactive stores.
//

export class SuibaseGlobalStates {
  public loaded: boolean = false;

  public uiSelectedContext: string = "DSUI";
  public uiSelectedContextCallback: (newUiSelectedContext: string) => void = (
    // eslint-disable-next-line
    _newUiSelectedContext: string
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

  public static activateForExtension() {
    console.log("SuibaseData.activateForExtension() called");
    if (SuibaseData.instance) {
      console.log("Error: SuibaseData.activateForExtension() called more than once");
      return;
    }

    SuibaseData.getInstance(); // Create the singleton
  }

  public static activateForView() {
    console.log("SuibaseData.activateForView() called");
    if (SuibaseData.instance) {
      console.log("Error: SuibaseData.activateForView() called more than once");
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
