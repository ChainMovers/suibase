import * as vscode from "vscode";
import { BaseWebview } from "../bases/BaseWebview";

export class SuibaseSidebar extends BaseWebview {
  private static instance?: SuibaseSidebar;

  private constructor() {
    super("suibase.sidebar", "Sui Sidebar");
  }

  public static activate(context: vscode.ExtensionContext) {
    console.log("SuibaseSidebar.activate() called");
    if (SuibaseSidebar.instance) {
      console.log("Error: SuibaseSidebar.activate() called more than once");
      return;
    }

    SuibaseSidebar.instance = new SuibaseSidebar();

    // Tell VSCode how to build the view using the SuibaseSidebar::BaseWebview::WebviewViewProvider
    let explorerView = vscode.window.registerWebviewViewProvider("explorerView", SuibaseSidebar.instance, {
      webviewOptions: {
        retainContextWhenHidden: true,
      },
    });

    context.subscriptions.push(explorerView);
  }

  public static deactivate() {
    if (SuibaseSidebar.instance) {
      SuibaseSidebar.instance.dispose();
      delete SuibaseSidebar.instance;
      SuibaseSidebar.instance = undefined;
    } else {
      console.log("Error: SuibaseSidebar.deactivate() called out of order");
    }
  }

  // Dispose is a callback triggered by VSCode (see BaseWebview).
  protected dispose() {
    console.log("SuibaseSidebar.dispose() called");
    if (SuibaseSidebar.instance) {
      super.dispose();
      delete SuibaseSidebar.instance;
      SuibaseSidebar.instance = undefined;
    } else {
      console.log("Error: dispose() called out of order");
    }
  }
}
