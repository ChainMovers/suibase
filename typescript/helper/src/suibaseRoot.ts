import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export class SuibaseRoot {
  private installed = false;
  readonly suibasePath: string;
  readonly workdirsPath: string;

  constructor(rootPath?: string) {
    this.suibasePath = rootPath ?? join(homedir(), "suibase");
    this.workdirsPath = join(this.suibasePath, "workdirs");
    this.refreshState();
  }

  refreshState(): void {
    const baseOk = this.suibasePath.length > 0 && existsSync(this.suibasePath);
    const wdOk =
      this.workdirsPath.length > 0 && existsSync(this.workdirsPath);
    this.installed = baseOk && wdOk;
  }

  isInstalled(): boolean {
    this.refreshState();
    return this.installed;
  }
}
