// These are all the commands that can be trigger either
// through the VSCode command palette (Ctrl-Shift-P) or
// a UI interaction (e.g. pressing a "refresh" button).
//
import * as vscode from "vscode";
import { SuibaseExec } from "./suibaseExec";

import { DashboardPanel } from "./panels/DashboardPanel";

export class SuibaseCommands {
  private static instance: SuibaseCommands | undefined;
  private static context: vscode.ExtensionContext | undefined;

  private constructor() {} // activate() does the instantiation instead.

  public static activate(context: vscode.ExtensionContext) {
    if (!typeof SuibaseCommands.context === undefined) {
      console.log("Error: SuibaseCommands.activate() called more than once");
      return;
    }

    SuibaseCommands.context = context;
    SuibaseCommands.instance = new SuibaseCommands();

    {
      let disposable = vscode.commands.registerCommand("suibase.settings", () => {
        SuibaseCommands.getInstance()?.settings();
      });
      context.subscriptions.push(disposable);
    }

    {
      let disposable = vscode.commands.registerCommand("suibase.refresh", () => {
        SuibaseCommands.getInstance()?.refresh();
      });
      context.subscriptions.push(disposable);
    }

    // Call the refresh command periodically.
    setInterval(() => {
      vscode.commands.executeCommand("suibase.refresh");
    }, 3000); // 3 seconds

    return SuibaseCommands.instance;
  }

  public static deactivate() {
    delete SuibaseCommands.instance;

    if (SuibaseCommands.context) {
      SuibaseCommands.context = undefined;
    } else {
      console.log("Error: SuibaseCommands.deactivate() called more than once");
    }
  }

  public static getInstance(): SuibaseCommands | undefined {
    if (!SuibaseCommands.instance) {
      console.log("Error: SuibaseExec.getInstance() called before activate()");
    }
    return SuibaseCommands.instance;
  }

  public refresh(workdir?: string) {
    //const str = "SuibaseCommands.refresh() called";
    //console.log(str);

    // TODO Debouncing to avoid excessive global refresh.

    // Do a JSON-RPC call to the suibase server API.
    //
    // If workdir is not specified, update them all.

    // This is a best-effort request and reactions to the
    // eventual response are handled somewhere else...
    SuibaseExec.getInstance()?.getLinks();
  }

  public settings() {
    if (!SuibaseCommands.context) {
      return;
    }
    DashboardPanel.render(SuibaseCommands.context.extensionUri);
  }
}
