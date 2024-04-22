/* eslint-disable @typescript-eslint/no-explicit-any */
// The purpose of the SuibaseJSONStorage is to :
//    - Compare very quickly two JSON storage and optionally update the storage.
//    - Trigger the 'deltaDetected' callback.
//
// This is a base class that handle "json" as a whole. Derived classes
// interpret the JSON for finer grained handling.

export class SuibaseJson {
  // A change of method UUID means that delta detection using the dataUUID is
  // not valid.
  //
  // Therefore, delta should be done by comparing the data as a whole.
  public methodUUID: string;

  // Allows to quickly detects delta. This is a time sortable UUID, therefore
  // an update using a lower dataUUI should be ignored (out of order).
  public dataUUID: string;

  public json: any;

  // Constructor
  constructor() {
    this.methodUUID = "";
    this.dataUUID = "";
    this.json = null;
  }

  // Return true if the json has changed.
  public update(methodUUID: string, dataUUID: string, json: any): boolean {
    if (this.json === null || this.methodUUID !== methodUUID || dataUUID > this.dataUUID) {
      this.methodUUID = methodUUID;
      this.dataUUID = dataUUID;
      this.json = json;
      this.deltaDetected();
      return true;
    }
    return false;
  }

  protected deltaDetected() {
    // Callback handled by a derived class when a delta is detected.
    //console.log(`SuibaseJson.deltaDetected() called for ${JSON.stringify(this.json)}`);
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
