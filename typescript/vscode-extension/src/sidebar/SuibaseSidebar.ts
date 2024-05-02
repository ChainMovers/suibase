import * as vscode from "vscode";
import { BaseWebview } from "../bases/BaseWebview";
import { SuibaseData } from "../common/SuibaseData";
import { WEBVIEW_EXPLORER } from "../common/Consts";

export class SuibaseSidebar extends BaseWebview {
  private static instance?: SuibaseSidebar;

  private constructor() {
    super(WEBVIEW_EXPLORER, "Sui Sidebar");
  }

  public static activate(context: vscode.ExtensionContext) {
    console.log("SuibaseSidebar.activate() called");
    if (SuibaseSidebar.instance) {
      console.log("Error: SuibaseSidebar.activate() called more than once");
      return;
    }

    SuibaseSidebar.instance = new SuibaseSidebar();

    // Tell VSCode how to build the view using the SuibaseSidebar::BaseWebview::WebviewViewProvider
    const explorerView = vscode.window.registerWebviewViewProvider("explorerView", SuibaseSidebar.instance, {
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

  // Override BaseWebview::handleMessage

  protected handleMessage(message: any): void {
    //console.log("SuibaseSidebar.handleMessage() called");
    //console.log(message);
    const sbData = SuibaseData.getInstance();
    switch (message.type) {
      case "init-view":
        super.postMessage({ type: "init-global-states", message: sbData.globalStates.serialize() });
        // TODO Initialize the other states...
        break;
      // TODO Implement new message types to handle workdir states.
      // TODO Implement new message types to handle console states.
      // TODO Implement new message types to handle wallet states.
    }
  }
}
