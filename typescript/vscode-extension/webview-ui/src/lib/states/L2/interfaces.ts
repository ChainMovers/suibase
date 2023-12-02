import type { Readable } from "svelte/store";
import type {
  EpochLatest,
  EpochLeaderboard,
  EpochValidators,
  VersionsLatest,
  WorkdirStatus,
} from "../L1/json-constructed";
import type { IEpochETA } from "../L1/poc-interfaces";

export interface IEpochStores {
  epoch_latest: Readable<EpochLatest>;
  epoch_leaderboard: Readable<EpochLeaderboard>;
  epoch_leaderboard_header: Readable<IEpochETA>;
  epoch_validators: Readable<EpochValidators>;
  epoch_validators_header: Readable<IEpochETA>;
  update_ev(force_refresh: boolean): Promise<void>;
}

export interface EpochStoresConstructor {
  new (context: IBlockchainContext): IEpochStores;
}

export function createEpochStores(ctor: EpochStoresConstructor, context: IBlockchainContext): IEpochStores {
  return new ctor(context);
}

export interface IGlobalsStores {
  versions_latest: Readable<VersionsLatest>;
  workdir_status: Readable<WorkdirStatus>;
  update_versions(force_refresh: boolean): Promise<void>;
}

export interface GlobalsStoresConstructor {
  new (context: IBlockchainContext): IGlobalsStores;
}

export function createGlobalsStores(
  ctor: GlobalsStoresConstructor,
  context: IBlockchainContext
): IGlobalsStores {
  return new ctor(context);
}

export interface IBlockchainContext {
  readonly ui_selector_name: string; // "Sui (Mainnet)", "Sui (Localnet)", "BTC (Devnet)"...
  readonly ui_name: string; // Sui, Bitcoin, Ethereum...
  readonly symbol: string; // SUI,BTC,ETH...
  readonly prefix: string; // X_CONTEXT_PREFIX is short unique name intended to never change.
  readonly workdir: string; // localnet, devnet, testnet or mainnet.
  readonly server: string; // Backend API server.
}

export interface IBlockchainStores {
  readonly epoch_stores: IEpochStores; // Svelte stores updated typically once per epoch.
  readonly globals_stores: IGlobalsStores; // Svelte stores updated to match the backend.
}

export interface IBlockchainContextMapping {
  readonly context: IBlockchainContext;
  readonly stores: IBlockchainStores;
}
