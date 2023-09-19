// These are all the commands that can be trigger either
// through the VSCode command palette (Ctrl-Shift-P) or
// a UI interaction (e.g. pressing a "refresh" button).
//
import * as vscode from "vscode";

export class SuibaseCommands {
  private static instance: SuibaseCommands | undefined;
  private static context: vscode.ExtensionContext | undefined;

  private constructor() {} // activate() does the instantion instead.

  public static activate(context: vscode.ExtensionContext) {
    if (!typeof SuibaseCommands.context === undefined) {
      console.log("Error: SuibaseCommands.activate() called more than once");
      return;
    }

    SuibaseCommands.context = context;
    SuibaseCommands.instance = new SuibaseCommands();

    {
      let disposable = vscode.commands.registerCommand(
        "suibase.settings",
        () => {
          SuibaseCommands.getInstance().settings();
        }
      );
      context.subscriptions.push(disposable);
    }

    {
      let disposable = vscode.commands.registerCommand(
        "suibase.refresh",
        () => {
          SuibaseCommands.getInstance().refresh();
        }
      );
      context.subscriptions.push(disposable);
    }

    return SuibaseCommands.instance;
  }

  public static deactivate() {
    if (typeof SuibaseCommands.context === undefined) {
      console.log("Error: SuibaseCommands.deactivate() called more than once");
    }
    SuibaseCommands.context = undefined;
    delete SuibaseCommands.instance;
  }

  public static getInstance(): SuibaseCommands {
    if (!SuibaseCommands.instance) {
      console.log("Error: SuibaseExec.getInstance() called before activate()");
      SuibaseCommands.instance = new SuibaseCommands();
    }
    return SuibaseCommands.instance;
  }

  public refresh() {
    const str = "SuibaseCommands.refresh() called";
    vscode.window.showInformationMessage(str);
    console.log(str);
  }

  public settings() {
    const str = "SuibaseCommands.settings() called";
    vscode.window.showInformationMessage(str);
    console.log(str);
  }
}
