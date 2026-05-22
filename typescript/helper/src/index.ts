// suibase
//
// API to suibase intended for development of Sui tool/test automation
// (Playwright, Vitest, ...) and TypeScript-based backends.
//
// See https://suibase.io for more info.

import { SuibaseError } from "./error.js";
import { SuibaseRoot } from "./suibaseRoot.js";
import { SuibaseWorkdir } from "./suibaseWorkdir.js";

export { SuibaseError } from "./error.js";
export type { SuibaseErrorCode } from "./error.js";

export interface HelperOptions {
  /**
   * Override the suibase installation directory. Defaults to `~/suibase`.
   * Mostly useful for testing.
   */
  rootPath?: string;
}

/**
 * A lightweight API to suibase. Multiple instances can be created within the same app.
 *
 * Usage:
 *   1. Call `isInstalled()` to confirm suibase is available.
 *   2. Call `selectWorkdir()` to pick a workdir (`active`, `localnet`, …).
 *   3. Call any of the other methods. Most relate to the selected workdir.
 */
export class Helper {
  private readonly root: SuibaseRoot;
  private workdir_?: SuibaseWorkdir;

  constructor(options: HelperOptions = {}) {
    this.root = new SuibaseRoot(options.rootPath);
  }

  isInstalled(): boolean {
    return this.root.isInstalled();
  }

  selectWorkdir(workdirName: string): void {
    const wd = new SuibaseWorkdir();
    wd.initFromExisting(this.root, workdirName);
    this.workdir_ = wd;
  }

  workdir(): string {
    return this.requireWorkdir().getName();
  }

  keystorePathname(): string {
    return this.requireWorkdir().keystorePathname(this.root);
  }

  packageObjectId(packageName: string): string {
    return this.requireWorkdir().packageObjectId(this.root, packageName);
  }

  packageId(packageName: string): string {
    return this.packageObjectId(packageName);
  }

  publishedNewObjectIds(objectType: string): string[] {
    return this.requireWorkdir().publishedNewObjectIds(this.root, objectType);
  }

  publishedNewObjects(objectType: string): string[] {
    return this.publishedNewObjectIds(objectType);
  }

  clientSuiAddress(addressName: string): string {
    return this.requireWorkdir().clientSuiAddress(this.root, addressName);
  }

  clientAddress(addressName: string): string {
    return this.clientSuiAddress(addressName);
  }

  rpcUrl(): string {
    return this.requireWorkdir().rpcUrl(this.root);
  }

  wsUrl(): string {
    return this.requireWorkdir().wsUrl(this.root);
  }

  private requireWorkdir(): SuibaseWorkdir {
    if (!this.workdir_) {
      throw new SuibaseError(
        "WorkdirNotSelected",
        "Workdir not selected. Successful call to `selectWorkdir` needed",
      );
    }
    return this.workdir_;
  }
}
