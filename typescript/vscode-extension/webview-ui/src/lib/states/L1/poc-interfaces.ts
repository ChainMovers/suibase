// Some trivial interfaces with no dependencies.
export interface ILoadedState {
  readonly isLoaded: boolean;
}

export class JSONHeader {
  readonly method: string;
  readonly methodUuid: string;
  readonly dataUuid: string;
  readonly key: string;

  constructor(method: string = "", method_uuid: string = "0", data_uuid: string = "0", key: string = "") {
    this.method = method;
    this.methodUuid = method_uuid;
    this.dataUuid = data_uuid;
    this.key = key;
  }

  static default(): JSONHeader {
    return new JSONHeader();
  }

  public is_older_revision_of(other?: JSONHeader): boolean {
    if (other === undefined) {
      return false; // Other is not initialized, lets assume it is not newer.
    }

    // If their method_uuid are different, then assume other is newer.
    // (it is not possible to know which version is more recent).
    if (other.methodUuid != this.methodUuid) {
      return true;
    }

    // Both are defined, so compare the sortable data_uuid.
    return other.dataUuid > this.dataUuid;
  }
}

export interface IVersioned {
  // header.data_uuid can be compared for when their uuid_method is the same.
  // A greater uuid_data means more recent data.
  //
  // header.method_uuid is not sortable and should be used only to detect a change of producer.
  readonly header: JSONHeader;
}

export interface IEpochRevision {
  readonly e: number; // Epoch number.
  readonly r: number; // Revision number.
  readonly s: string; // The corresponding unique hash string.
}

export interface IEndOfEpochFields {
  readonly e: number; // Epoch number.
  readonly t: number; // Unix timestamp (UTC) for beginning of epoch.
  readonly y: number; // Unix timestamp (UTC) for end of epoch estimation.
}

export interface IEpochRevision2 {
  readonly e: number; // Epoch number.
  readonly r2: number; // Revision wave 2 number.
  readonly s2: string; // The corresponding unique hash string.
}

export interface IContextKeyed {
  readonly context_key: string;
}

export interface IEndOfEpochETA {
  readonly e: number; // Epoch number.
  readonly remaining: string; // Time to end of epoch.
  readonly elapsed: string; // Time since beginning of epoch.
  readonly tick: number; // Increment on every update (debug purpose).
}

export type IEpochETA = IEndOfEpochETA & ILoadedState & IContextKeyed;
