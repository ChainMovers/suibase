// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";

import { SuibaseSidebar } from "./sidebar/SuibaseSidebar";
import { SuibaseExec } from "./SuibaseExec";
import { SuibaseCommands } from "./SuibaseCommands";
import { BaseWebview } from "./bases/BaseWebview";
import { SuibaseData } from "./common/SuibaseData";
import { BackendSync } from "./BackendSync";

// This method is called when the extension is activated by VSCode.
export function activate(context: vscode.ExtensionContext) {
  // Instantiate all the singleton instances.
  // Each will perform their own registrations.

  // Low-level APIs
  SuibaseData.activate(); // Only state/status storage (no app logic).
  SuibaseExec.activate(context); // Used to perform shell commands.
  BaseWebview.activate(context); // Base class for all webview.

  // MUST be activated after BaseWebview (because of callback initialization).
  // MUST be activated before SuibaseSidebar (so it is ready before UI interaction).
  //
  // Data flows are:
  //  suibase-daemon --HTTP JSON-RPC--> BackendSync ---(update messages)---> Webview  --> React States
  //                                    BackendSync <--(request messages)--  Webview  <-- React States
  BackendSync.activate();

  // "App logic" enabled next.
  SuibaseCommands.activate(context);

  // Make main UI controller visible (with default unloaded data).
  SuibaseSidebar.activate(context);

  // Enable getting states from the backend.

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
  BaseWebview.deactivate();
  SuibaseExec.deactivate();

  console.log("extension deactivate() completed");
}
