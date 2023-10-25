// These are all the commands that can be trigger either
// through the VSCode command palette (Ctrl-Shift-P) or
// a UI interaction (e.g. pressing a "refresh" button).
//
import * as vscode from "vscode";
import { SuibaseExec } from "./SuibaseExec";

import { DashboardPanel } from "./panels/DashboardPanel";
import { ConsolePanel } from "./panels/ConsolePanel";

export class SuibaseCommands {
  private static instance?: SuibaseCommands;
  private static context?: vscode.ExtensionContext;

  private constructor() {} // Called from activate() only.
  private dispose() {} // Called from deactivate() only.

  public static activate(context: vscode.ExtensionContext) {
    if (SuibaseCommands.instance) {
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
      let disposable = vscode.commands.registerCommand("suibase.console", () => {
        SuibaseCommands.getInstance()?.console();
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
  }

  public static deactivate() {
    if (SuibaseCommands.instance) {
      SuibaseCommands.instance.dispose();
      delete SuibaseCommands.instance;
      SuibaseCommands.instance = undefined;
    } else {
      console.log("Error: SuibaseCommands.deactivate() called out of order");
    }

    SuibaseCommands.context = undefined;
  }

  public static getInstance(): SuibaseCommands | undefined {
    if (!SuibaseCommands.instance) {
      console.log("Error: SuibaseCommands.getInstance() called before activate()");
      return undefined;
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
    DashboardPanel.render();
  }

  public console() {
    if (!SuibaseCommands.context) {
      return;
    }
    ConsolePanel.render();
  }
}
