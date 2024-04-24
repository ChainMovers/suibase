// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";

import { SuibaseSidebar } from "./sidebar/SuibaseSidebar";
import { SuibaseExec } from "./SuibaseExec";
import { SuibaseCommands } from "./SuibaseCommands";
import { BaseWebview } from "./bases/BaseWebview";
import { SuibaseData } from "./common/SuibaseData";
import { BackendSync } from "./BackendSync";

// This method is called *once* when the extension is activated by VSCode.
export function activate(context: vscode.ExtensionContext) {
  // Instantiate all the singleton instances.
  // Each will perform their own registrations.

  // Low-level APIs
  SuibaseData.activate(); // Some global state/status storage (no app logic).
  SuibaseExec.activate(context); // Shell commands, JSON-RPC call and websocket subscribe with suibase-daemon.
  BaseWebview.activate(context); // Base class for all webview.

  // BackendSync
  //
  // Periodic and on-demand forward of data between backend suibase-daemon and webview(s).
  //
  // MUST be activated after BaseWebview (because of callback initialization).
  // MUST be activated before SuibaseSidebar (so it is ready before UI interaction).
  //
  // Data flows:
  //  suibase-daemon ---(JSON-RPC)--> BackendSync ---(window.message)--> Webview --> React States Update
  //                 <--(JSON-RPC)--- BackendSync <--(window.message)--- Webview <-- React States/Effects
  //
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
