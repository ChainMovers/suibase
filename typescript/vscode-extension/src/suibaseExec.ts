// An API that encapsulate Suibase CLI calls (e.g. "lsui", "localnet" etc...)
import * as vscode from "vscode";
import * as cp from "child_process";

const execShell = (cmd: string) =>
  new Promise<string>((resolve, reject) => {
    cp.exec(cmd, (err, out) => {
      if (err) {
        return reject(err);
      }
      return resolve(out);
    });
  });

export class SuibaseExec {
  private static instance: SuibaseExec | undefined;
  private static context: vscode.ExtensionContext | undefined;

  private constructor() {} // activate() does the instantion instead.

  public static activate(context: vscode.ExtensionContext) {
    if (!typeof SuibaseExec.context === undefined) {
      console.log("Error: SuibaseExec.activate() called more than once");
      return;
    }

    SuibaseExec.context = context;
    SuibaseExec.instance = new SuibaseExec();

    return SuibaseExec.instance;
  }

  public static deactivate() {
    if (typeof SuibaseExec.context === undefined) {
      console.log("Error: SuibaseSidebar.deactivate() called more than once");
    }
    SuibaseExec.context = undefined;
    delete SuibaseExec.instance;
  }

  public static getInstance(context: vscode.ExtensionContext): SuibaseExec {
    if (!SuibaseExec.instance) {
      console.log("Error: SuibaseExec.getInstance() called before activate()");
      SuibaseExec.instance = new SuibaseExec();
    }
    return SuibaseExec.instance;
  }

  public async version(): Promise<string> {
    try {
      const result = await execShell("localnet --version");
      console.log(result);
      return Promise.resolve(result);
    } catch (err) {
      return Promise.reject(err);
    }
  }
}
