/* eslint-disable @typescript-eslint/no-explicit-any */
// The purpose of the SuibaseJson is to :
//    - Compare very quickly two JSON storage and optionally update the storage.
//    - Detect delta using UUID.
//
// This is a base class that handle "json" as a whole. Derived classes
// interpret the JSON for finer grained handling.

export class SuibaseJson {
  private method: string; // The method used to get this JSON from backend (e.g. "getWorkdirStatus")

  // A change of method UUID means that delta detection using the dataUuid is
  // not valid.
  //
  // Therefore, delta should be done by comparing the data as a whole.
  private methodUuid: string;

  // Allows to quickly detects delta. This is a time sortable UUID, therefore
  // an update using a lower dataUUI should be ignored (out of order).
  private dataUuid: string;

  private json: any;

  // Constructor
  constructor() {
    this.method = "";
    this.methodUuid = "";
    this.dataUuid = "";
    this.json = null;
  }

  // Getters for every private.
  public getMethod(): string {
    return this.method;
  }

  public getMethodUuid(): string {
    return this.methodUuid;
  }

  public getDataUuid(): string {
    return this.dataUuid;
  }

  public getJson(): any {
    return this.json;
  }

  public update(json: any): boolean {
    const method = json.header.method;
    const methodUuid = json.header.methodUuid;
    const dataUuid = json.header.dataUuid;

    // TODO A lot more of data validation here...

    if (this.method !== "" && method !== this.method) {
      // Caller is mixing JSON responses from different methods. Likely a software bug.
      console.error(`Trying to update [${this.json.method}] using a JSON from [${method}]`);
      return false;
    }

    if (this.json === null || this.methodUuid !== methodUuid || dataUuid > this.dataUuid) {
      this.method = method;
      this.methodUuid = methodUuid;
      this.dataUuid = dataUuid;
      this.json = json;
      this.deltaDetected();
      return true;
    }
    return false;
  }

  protected deltaDetected() {
    // Callback handled by a derived class when a delta is detected.
  }
}

export class SuibaseJsonVersions extends SuibaseJson {
  // Stores the JSON returned by the getVersions backend.

  // Verify if this object element version is newer than the param.
  //
  //
  // Return true if the SuibaseJson param is *older* or show any sign of needing to be updated.
  // Return false if the SuibaseJson param is *same* or *newer* (or in some error cases).
  //
  // When true, the newer methodUuid and dataUuid expected is returned.
  public isWorkdirStatusUpdateNeeded(candidate: SuibaseJsonWorkdirStatus): [boolean, string, string] {
    // Get the getWorkdirStatus UUIDs.
    // Example of this.json:
    //     {"header":{"method":"getVersions","methodUuid":"...","dataUuid":"...","key":"localnet"},
    //      "versions":[{"method":"getWorkdirStatus","methodUuid":"...","dataUuid":"...","key":"localnet"}],
    //      "asuiSelection":"localnet"
    //     }
    // Iterate this.json.versions elements, and look for the method. Compare the methodUuid and dataUuid.

    try {
      const candidateShouldBeUpdated =
        candidate === null ||
        candidate.getJson() === null ||
        candidate.getMethod() === "" ||
        candidate.getMethodUuid() === "" ||
        candidate.getDataUuid() === "";
      //console.log(`candidateShouldBeUpdated: ${candidateShouldBeUpdated} method: ${candidate.getMethod()}`);
      for (const version of this.getJson().versions) {
        if (version.method === "getWorkdirStatus") {
          const methodUuid = version.methodUuid;
          const dataUuid = version.dataUuid;
          if (
            candidateShouldBeUpdated ||
            candidate.getMethodUuid() !== methodUuid ||
            candidate.getDataUuid() < dataUuid
          ) {
            return [true, methodUuid, dataUuid];
          }
          break;
        }
      }
    } catch (error) {
      console.error(
        `Problem comparing versions for SuibaseJsonWorkdirStatus ${JSON.stringify(
          candidate
        )} and versions ${JSON.stringify(this.getJson())}: error [${error}]`
      );
    }
    // Normal because candidate is same or not latest... or could be an error...
    return [false, "", ""];
  }

  protected deltaDetected() {
    /* Do nothing for now */
    super.deltaDetected();
  }
}

export class SuibaseJsonWorkdirStatus extends SuibaseJson {
  public status: string;
  public suiClientVersion: string;
  public suiClientVersionShort: string;
  public isLoaded;

  constructor() {
    super();
    this.status = "";
    this.suiClientVersion = "";
    this.suiClientVersionShort = "";
    this.isLoaded = false;
  }
  protected deltaDetected() {
    super.deltaDetected();
    try {
      this.status = this.getJson().status;
      this.suiClientVersion = this.getJson().clientVersion;
      if (typeof this.suiClientVersion === "string" && this.suiClientVersion.length > 0) {
        this.suiClientVersionShort = this.suiClientVersion.split("-")[0];
      } else {
        this.suiClientVersionShort = "";
      }
      this.isLoaded = true;
    } catch (error) {
      console.error(`Problem with SuibaseJsonWorkdirStatus loading: ${error}`);
    }
  }
}

// This is to be used internally by SuibaseJSONStorage only.
/*
class StorageValue {
  public suibaseJson: SuibaseJson;
  public onChangeCallbacks: Array<SuibaseJsonCallback>;
  // Constructor
  constructor(suibaseJson: SuibaseJson) {
    this.suibaseJson = suibaseJson;
    this.onChangeCallbacks = [];
  }
}*/

/*
export class SuibaseJSONStorage {
  // Map key string to SuibaseJson elements.

  private map: Map<string, StorageValue>;

  // Constructor
  constructor() {
    this.map = new Map<string, StorageValue>();
  }

  // Get a JSON element from the map.
  // If not in the map, then create a SuibaseJson with the
  // default for the requested type. This default is also
  // added to the map.
  public get(type: string): SuibaseJson {
    const found = this.map.get(type);
    if (found) {
      return found.suibaseJson;
    }
    return this.addDefaultElement(type).suibaseJson;
  }

  // Add a JSON element to the map.
  // Replace an existing one only if the version is higher.
  public set(newJSON: SuibaseJson) {
    const mappedElement = this.map.get(newJSON.type);
    if (mappedElement) {
      const mappedJSON = mappedElement.suibaseJson;
      if (mappedJSON.version < newJSON.version) {
        mappedJSON.update(newJSON);
        mappedElement.onChangeCallbacks.forEach((callback) => callback(mappedJSON));
      }
    } else {
      // Creating the default element first just to update it after is not the most efficient,
      // but it reduces initialization sequence variations (and need fro more tests).
      const newMappedElement = this.addDefaultElement(newJSON.type);
      newMappedElement.suibaseJson.update(newJSON);
      // Note: new element created here... no callback possibly added yet.
    }
  }

  // Add a callback for a given type.
  // If not in the map, then create a SuibaseJson with the
  // default for the requested type. This default is also
  // added to the map.
  public addCallback(type: string, onChange: SuibaseJsonCallback) {
    let mappedElement = this.map.get(type);
    if (!mappedElement) {
      mappedElement = this.addDefaultElement(type);
    }
    mappedElement.onChangeCallbacks.push(onChange);
    onChange(mappedElement.suibaseJson);
  }

  private addDefaultElement(type: string): StorageValue {
    const newMappedJSON = new SuibaseJson(type, 0, "");
    const newMappedElement = new StorageValue(newMappedJSON);
    this.map.set(type, newMappedElement);
    return newMappedElement;
  }
}
*/
