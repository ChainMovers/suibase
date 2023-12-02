// Class acting as read-only containers of what was received from the network.
//
// Can be initialized only from JSON:
//   - Perform some security validations.
//   - Property names must match what is being sent by backend\views.py
//
// Intended to be used as stored values for svelte stores.

import {
  IContextKeyed,
  ILoadedState,
  IEpochRevision,
  IEpochRevision2,
  IVersioned,
  JSONHeader,
} from "./poc-interfaces";

interface JsonRpc2Response {
  jsonrpc: string;
  result: any;
  id: string | number | null;
}

export class JSONConstructed {
  public readonly json_result: unknown = "";
  public readonly json_id: number = 0;

  constructor(json_obj: unknown) {
    // Extract the 'result' field into json_result. If any error, then throw an exception.
    if (json_obj == undefined) {
      throw new Error("constructor json_obj undefined");
    }
    const json_rpc_resp = json_obj as JsonRpc2Response;

    // Validate that every fields are present.
    const jsonrpc = json_rpc_resp.jsonrpc;
    if (jsonrpc == undefined) {
      throw new Error("constructor jsonrpc field undefined");
    }

    // Sanity check the 'jsonrpc' version field.
    const jsonrpc_pattern = /^\d+\.\d+$/;
    if (!jsonrpc_pattern.test(jsonrpc)) {
      throw new Error("constructor jsonrpc field invalid {}" + JSON.stringify(json_obj));
    }

    // Sanity check that 'id' is not null and convert to number as needed.
    const id = json_rpc_resp.id;
    if (id == undefined) {
      throw new Error("constructor id field undefined");
    }
    if (id == null) {
      throw new Error("constructor id field null");
    }
    if (typeof id === "string") {
      this.json_id = parseInt(id);
    } else {
      this.json_id = id;
    }
    // Sanity check that 'result' is present.
    const result = json_rpc_resp.result;
    if (result == undefined) {
      throw new Error("constructor result field undefined");
    }

    // Blindly extract the result field. Derived classes
    // will validate/interpret it further...
    this.json_result = result;
  }

  toString(): string {
    return Object.prototype.toString.call(this) + " = " + JSON.stringify(this);
  }

  // Validate and extract the header field from json_result.
  protected extract_header(expected_method: string, expected_key: string): JSONHeader {
    if (this.json_result == undefined) {
      throw new Error("Missing json_result");
    }

    const header = this.json_result["header"];
    return this.convert_to_JSONHeader(header, expected_method, expected_key);
  }

  protected convert_to_JSONHeader(
    header_obj: unknown,
    expected_method: string = undefined,
    expected_key: string = undefined
  ): JSONHeader {
    // This is the app specific header expected from the backend server.
    if (header_obj == undefined) {
      throw new Error("Missing header: " + this.json_result);
    }

    const json_header = header_obj as JSONHeader;
    if (json_header.method == undefined) {
      throw new Error("Missing header.method: " + this.json_result);
    }
    if (expected_method && json_header.method != expected_method) {
      throw new Error(
        "Unexpected header.method [" +
          json_header.method +
          "], but expect [" +
          expected_method +
          "]" +
          this.json_result
      );
    }

    if (json_header.key == undefined) {
      throw new Error("Missing header.key: " + this.json_result);
    }
    if (expected_key && json_header.key != expected_key) {
      throw new Error(
        "Unexpected header.key [" +
          json_header.key +
          "], but expect [" +
          expected_key +
          "]: " +
          this.json_result
      );
    }

    // Verify that method_uuid and data_uuid are non-empty strings.
    if (json_header.methodUuid == undefined) {
      throw new Error("Missing header.methodUuid: " + this.toString());
    }
    if (json_header.methodUuid == "") {
      throw new Error("Empty header.methodUuid: " + this.json_result);
    }

    if (json_header.dataUuid == undefined) {
      throw new Error("Missing header.dataUuid: " + this.json_result);
    }
    if (json_header.dataUuid == "") {
      throw new Error("Empty header.dataUuid: " + this.json_result);
    }

    // TODO Further validate the UUID fields.
    return new JSONHeader(json_header.method, json_header.methodUuid, json_header.dataUuid, json_header.key);
  }
}

export class VersionsLatest extends JSONConstructed implements IVersioned, IContextKeyed, ILoadedState {
  get [Symbol.toStringTag]() {
    return "VersionsLatest";
  }
  readonly isLoaded: boolean; // True if successfully initialized with JSON.
  readonly header: JSONHeader;
  readonly context_key: string; // The context.prefix used as a unique key.
  readonly versions: JSONHeader[];
  readonly version_workdir_status_idx: number;

  constructor(json_obj: unknown = undefined, workdir: string, context_key: string) {
    // Just throw an exception if anything goes wrong.
    super(json_obj); // Validate + initializes json_result with JSON-RPC result field.
    this.isLoaded = false;
    this.header = super.extract_header("getVersions", workdir);
    this.context_key = context_key;

    const candidate = this.json_result as VersionsLatest;

    this.versions = [];
    for (let i = 0; i < candidate.versions.length; i++) {
      const element = candidate.versions[i];
      this.versions.push(super.convert_to_JSONHeader(element));
    }

    // Quick check if something went wrong.
    if (this.versions.length == 0) {
      throw new Error("Empty versions: " + this.json_result);
    }

    // Initialize index on expected elements in versions.
    // Example of matching is:
    //   version_workdir_status_idx is the index in versions where the element has method = "GetWorkdirStatus".
    //   version_workdir_status_idx = -1 if not found.
    this.version_workdir_status_idx = this.versions.findIndex((element) => {
      return element.method == "getWorkdirStatus";
    });
    if (this.version_workdir_status_idx == -1) {
      throw new Error("Missing getWorkdirStatus: " + this.json_result);
    }

    // Success.
    this.isLoaded = true;
  }

  public isEquivalent(other: VersionsLatest): boolean {
    const areVersionsEqual =
      this.versions.length === other.versions.length &&
      this.versions.every((value, index) => value === other.versions[index]);

    // Purposely do not compare base properties.
    const result = this.isLoaded === other.isLoaded && this.header === other.header && areVersionsEqual;

    return result;
  }
}

export class WorkdirPackagesConfig
  extends JSONConstructed
  implements IVersioned, IContextKeyed, ILoadedState
{
  get [Symbol.toStringTag]() {
    return "WorkdirPackagesConfig";
  }
  readonly isLoaded: boolean; // True if successfully initialized with JSON.
  readonly header: JSONHeader;
  readonly context_key: string;

  constructor(json_obj: unknown = undefined, workdir: string, context_key: string) {
    // Just throw an exception if anything goes wrong.
    super(json_obj);
    this.isLoaded = false;
    this.header = super.extract_header("getWorkdirPackagesConfig", workdir);
    this.context_key = context_key;

    const candidate = json_obj as WorkdirPackagesConfig;

    // TODO More validation and loading into specialize readonly members...

    // Success.
    this.isLoaded = true;
  }

  public isEquivalent(other: WorkdirPackagesConfig): boolean {
    // Purposely do not compare base properties.
    return this.isLoaded === other.isLoaded && this.header === other.header;
  }
}

export class WorkdirStatus extends JSONConstructed implements IVersioned, IContextKeyed, ILoadedState {
  get [Symbol.toStringTag]() {
    return "WorkdirStatus";
  }
  readonly isLoaded: boolean; // True if successfully initialized with JSON.
  readonly header: JSONHeader;
  readonly context_key: string;

  constructor(json_obj: unknown = undefined, key: string) {
    // Just throw an exception if anything goes wrong.
    super(json_obj);
    this.context_key = key;
    if (json_obj == undefined) {
      this.isLoaded = false;
    } else {
      const candidate = json_obj as VersionsLatest;
      this.header = candidate.header;

      // TODO More validation and loading into specialize readonly members...

      // Success.
      this.isLoaded = true;
    }
  }

  public isEquivalent(other: WorkdirPackagesConfig): boolean {
    // Purposely do not compare base properties.
    return this.isLoaded === other.isLoaded && this.header === other.header;
  }
}

export class WorkdirSuiEvents extends JSONConstructed implements IVersioned, IContextKeyed, ILoadedState {
  get [Symbol.toStringTag]() {
    return "WorkdirSuiEvents";
  }
  readonly isLoaded: boolean; // True if successfully initialized with JSON.
  readonly header: JSONHeader;
  readonly context_key: string;
}

export class EpochLatest
  extends JSONConstructed
  implements IContextKeyed, IEpochRevision, IEpochRevision2, ILoadedState
{
  get [Symbol.toStringTag]() {
    return "EpochLatest";
  }

  readonly isLoaded: boolean; // True if successfully initialized with JSON.

  readonly f: number; // !=0 on server side errors.
  readonly v: number; // JSON format version.
  readonly e: number; // Epoch.
  readonly r: number; // Data revision for 1st wave.
  readonly t: number; // Unix timestamp (UTC). Start of epoch (network)
  readonly y: number; // Unix timestamp (UTC). End of epoch estimation.
  readonly z: number; // Number of revision for y.
  readonly s: string; // Hash suffix for 1st wave.
  readonly r2: number; // Data revision for 2nd wave.
  readonly s2: string; // Hash suffix for 2nd wave.
  readonly context_key: string;

  // Make sure to update isEqual if adding new property here!!!
  constructor(json_obj: unknown = undefined, key: string) {
    // Just throw an exception if anything goes wrong.
    super(json_obj);
    this.context_key = key;
    if (json_obj == undefined) {
      this.f = this.v = this.e = this.r = this.t = this.y = this.z = this.r2 = 0;
      this.s = this.s2 = "";
      this.isLoaded = false;
    } else {
      const candidate = json_obj as EpochLatest;
      this.f = candidate.f;
      this.v = candidate.v;
      this.e = candidate.e;
      this.r = candidate.r;
      this.t = candidate.t;
      this.y = candidate.y;
      this.z = candidate.z;
      this.s = candidate.s;
      this.r2 = candidate.r2;
      this.s2 = candidate.s2;

      // TODO More validation...
      if (this.f != 0) {
        throw new Error("Failed at server" + this.toString());
      }

      // Success.
      this.isLoaded = true;
    }
  }

  public isEquivalent(other: EpochLatest): boolean {
    // Purposely do not compare base properties.
    const result =
      this.isLoaded == other.isLoaded &&
      this.f == other.f &&
      this.v == other.v &&
      this.e == other.e &&
      this.r == other.r &&
      this.t == other.t &&
      this.y == other.y &&
      this.z == other.z &&
      this.s == other.s &&
      this.r2 == other.r2 &&
      this.s2 == other.s2;

    return result;
  }
}

class JSONConstructedEpochTable
  extends JSONConstructed
  implements IContextKeyed, IEpochRevision, ILoadedState
{
  readonly isLoaded: boolean; // True if successfully initialized with JSON.

  readonly f: number; // !=0 on server side errors.
  readonly v: number; // JSON format version.
  readonly e: number; // Epoch.
  readonly r: number; // Data revision for 1st wave.
  readonly t: number; // Unix timestamp (UTC). Start of epoch (network)
  readonly s: string; // Hash suffix for 1st wave.
  readonly row: number;
  readonly col: number;
  readonly table: unknown;
  readonly context_key: string;

  // Make sure to update isEqual if adding new property here!!!
  constructor(json_obj: unknown = undefined, key: string) {
    // Just throw an exception if anything goes wrong.
    super(json_obj);
    this.context_key = key;
    this.s = ""; // Not provided by API, but needed for IEpochRevision
    if (json_obj == undefined) {
      this.f = this.v = this.e = this.r = this.t = this.row = this.col = 0;
      this.table = {};
      this.isLoaded = false;
    } else {
      const candidate = json_obj as JSONConstructedEpochTable;
      this.f = candidate.f;
      this.v = candidate.v;
      this.e = candidate.e;
      this.r = candidate.r;
      this.t = candidate.t;
      this.row = candidate.row;
      this.col = candidate.col;
      this.table = candidate.table;

      // TODO More validation...
      if (this.f != 0) {
        throw new Error("Failed at server" + this.toString());
      }

      // Success.
      this.isLoaded = true;
    }
  }

  public isEquivalent(other: JSONConstructedEpochTable): boolean {
    // Purposely do not compare base properties and data.
    // If f v e r t, row and col matches then assume data is matching.
    const result =
      this.isLoaded == other.isLoaded &&
      this.f == other.f &&
      this.v == other.v &&
      this.e == other.e &&
      this.r == other.r &&
      this.t == other.t &&
      this.row == other.row &&
      this.col == other.col;

    return result;
  }
}

export class EpochLeaderboard extends JSONConstructedEpochTable {
  get [Symbol.toStringTag]() {
    return "EpochLeaderboard";
  }
}

export class EpochValidators extends JSONConstructedEpochTable {
  get [Symbol.toStringTag]() {
    return "EpochValidators";
  }
}
