// JSON storage of all data (config, state and status) related to a Suibase installation.
//
// Multiple instance are expected for a given data for doing comparison and detect deltas.
//
// The purpose of the SuibaseJSONStorage is to :
//    - Always get any data (even if not up to date) and revert to a default when not initialized.
//    - Allow to compare two JSON storage and optionally update a target.
//    - Trigger 'changes' callbacks.

type SuibaseJsonCallback = (suibaseJson: SuibaseJson) => void;

export class SuibaseJson {
  public type: string;
  public version: number;
  public data: string;

  // Constructor
  constructor(type: string, version: number, data: string) {
    this.type = type;
    this.version = version;
    this.data = data;
  }

  // Create a new SuibaseJSON from parsing a JSON string.
  // Return an error message (string type) on any parsing failure.
  public static fromString(jsonString: string): SuibaseJson | string {
    // TODO More validation.
    try {
      let json = JSON.parse(jsonString);
      return new SuibaseJson(json.type, json.version, json.data);
    } catch (e) {
      return `Error parsing JSON string: ${e} string: ${jsonString}`;
    }
  }

  public update(newJSON: SuibaseJson) {
    this.version = newJSON.version;
    this.data = newJSON.data;
  }
}

// This is to be used internally by SuibaseJSONStorage only.
class StorageValue {
  public suibaseJson: SuibaseJson;
  public onChangeCallbacks: Array<SuibaseJsonCallback>;
  // Constructor
  constructor(suibaseJson: SuibaseJson) {
    this.suibaseJson = suibaseJson;
    this.onChangeCallbacks = [];
  }
}

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
    let found = this.map.get(type);
    if (found) {
      return found.suibaseJson;
    }
    return this.addDefaultElement(type).suibaseJson;
  }

  // Add a JSON element to the map.
  // Replace an existing one only if the version is higher.
  public set(newJSON: SuibaseJson) {
    let mappedElement = this.map.get(newJSON.type);
    if (mappedElement) {
      let mappedJSON = mappedElement.suibaseJson;
      if (mappedJSON.version < newJSON.version) {
        mappedJSON.update(newJSON);
        mappedElement.onChangeCallbacks.forEach((callback) => callback(mappedJSON));
      }
    } else {
      // Creating the default element first just to update it after is not the most efficient,
      // but it reduces initialization sequence variations (and need fro more tests).
      let newMappedElement = this.addDefaultElement(newJSON.type);
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
    let newMappedJSON = new SuibaseJson(type, 0, "");
    let newMappedElement = new StorageValue(newMappedJSON);
    this.map.set(type, newMappedElement);
    return newMappedElement;
  }
}
