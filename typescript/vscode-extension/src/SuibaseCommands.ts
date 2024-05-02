// These are all the commands that can be trigger either
// through the VSCode command palette (Ctrl-Shift-P) or
// a UI interaction (e.g. pressing a "refresh" button).
//
import * as vscode from "vscode";

import { DashboardPanel } from "./panels/DashboardPanel";
import { ConsolePanel } from "./panels/ConsolePanel";
import { WEBVIEW_CONSOLE, WEBVIEW_DASHBOARD } from "./common/Consts";

export class SuibaseCommands {
  private static instance?: SuibaseCommands;
  private static context?: vscode.ExtensionContext;

  // eslint-disable-next-line @typescript-eslint/no-empty-function
  private constructor() {} // Called from activate() only.
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  private dispose() {} // Called from deactivate() only.

  public static activate(context: vscode.ExtensionContext) {
    if (SuibaseCommands.instance) {
      console.error("SuibaseCommands.activate() called more than once");
      return;
    }

    SuibaseCommands.context = context;
    SuibaseCommands.instance = new SuibaseCommands();

    {
      const disposable = vscode.commands.registerCommand(WEBVIEW_DASHBOARD, () => {
        SuibaseCommands.getInstance()?.settings();
      });
      context.subscriptions.push(disposable);
    }

    {
      const disposable = vscode.commands.registerCommand(WEBVIEW_CONSOLE, () => {
        SuibaseCommands.getInstance()?.console();
      });
      context.subscriptions.push(disposable);
    }
    /*
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
    */
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
