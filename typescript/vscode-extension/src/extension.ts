// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";

import { SuibaseSidebar } from "./SuibaseSidebar";
import { SuibaseExec } from "./SuibaseExec";
import { SuibaseCommands } from "./SuibaseCommands";
import { BasePanel } from "./panels/BasePanel";

// This method is called when the extension is activated by VSCode.
export function activate(context: vscode.ExtensionContext) {
  // Instantiate all the singleton instances.
  // Each will perform their own registrations.

  // Low-level APIs
  SuibaseExec.activate(context);
  BasePanel.activate(context);

  // "Business logic" enabled next.
  SuibaseCommands.activate(context);

  // UI elements enabled/visible last.
  SuibaseSidebar.activate(context);
  console.log("extension activate() completed");
}

// This method is called when the extension is deactivated by VSCode.
export function deactivate() {
  // Deactivate in reverse order of activation.

  // UI elements disabled/hidden first.
  SuibaseSidebar.deactivate();

  // "Business logic" disabled next.
  SuibaseCommands.deactivate();

  // Low-level APIs disabled last.
  BasePanel.deactivate();
  SuibaseExec.deactivate();

  console.log("extension deactivate() completed");
}
