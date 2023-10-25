import * as vscode from "vscode";
import { Disposable, WebviewPanel, window, Uri, ViewColumn } from "vscode";
import { getUri } from "../utilities/getUri";
import { getNonce } from "../utilities/getNonce";

/**
 * This class provides common functionality for all Suibase panel webview.
 *
 * Responsibilities include:
 * - Creating and rendering the webview.
 * - Setting the HTML, CSS and Typescript content of the webview panel
 * - Properly cleaning up and disposing of webview resources when the panel is closed
 * - Setting message listeners so data can be passed between the webview and extension
 */
export class BasePanel {
  // Create a map of BasePanel instances, each keyed by a unique string.
  // They all share the same extension context.
  //private static instances: Map<string, BasePanel>;
  private static context?: vscode.ExtensionContext;

  // Instance variables initialized in constructor()
  private readonly panelKey: string;
  private readonly panelTitle: string;
  private readonly extensionUri: Uri;

  // Instance variables initialized on first render()
  private panel: WebviewPanel | undefined;
  private disposables: Disposable[] = [];

  /**
   * The BasePanel class private constructor (called only from the render method).
   *
   * @param panel A reference to the webview panel
   * @param extensionUri The URI of the directory containing the extension
   */
  protected constructor(panelKey: string, panelTitle: string) {
    if (!BasePanel.context) {
      console.log("Error: BasePanel.constructor called before activate()");
      this.extensionUri = Uri.parse("file:///undefined");
    } else {
      this.extensionUri = BasePanel.context.extensionUri;
    }

    this.panelKey = panelKey;
    this.panelTitle = panelTitle;

    // Set an event listener to listen for messages passed from the webview context
    this._setWebviewMessageListener();
  }

  public static activate(context: vscode.ExtensionContext) {
    BasePanel.context = context;
  }

  public static deactivate() {
    BasePanel.context = undefined;
  }

  /**
   * Renders the current webview panel if it exists otherwise a new webview panel
   * will be created and displayed.
   *
   * @param extensionUri The URI of the directory containing the extension.
   * @param panelKey A unique string that identifies the webview panel.
   * @param panelTitle The title shown for this webview panel.
   */
  protected render() {
    // Look if there is already a BasePanel instance for the given key.
    if (this.panel !== undefined) {
      // If the webview panel already exists reveal it
      this.panel.reveal(ViewColumn.One);
      console.log("BasePanel _panel.reveal() called");
    } else {
      // If a webview panel does not already exist create and show a new one
      // "this" here is the subclass that extends BasePanel.
      this.panel = window.createWebviewPanel(
        this.panelKey, // Panel view type, must match what is in package.json
        this.panelTitle,
        // The editor column the panel should be displayed in
        ViewColumn.One,
        // Extra panel configurations
        {
          // Enable JavaScript in the webview
          enableScripts: true,
          // Restrict the webview to only load resources from the `out` and `webview-ui/build` directories
          localResourceRoots: [
            Uri.joinPath(this.extensionUri, "out"),
            Uri.joinPath(this.extensionUri, "webview-ui/public/build"),
            Uri.joinPath(this.extensionUri, "webview-ui/node_modules/@vscode/codicons/dist"),
          ],
        }
      );

      // Set an event listener to listen for when the panel is disposed (i.e. when the user closes
      // the panel or when the panel is closed programmatically)
      this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

      // Set the HTML content for the webview panel
      this.panel.webview.html = this._getWebviewContent();

      // Register message handling.
      this._setWebviewMessageListener();
    }
  }

  /**
   * Cleans up and disposes of webview resources when the webview panel is closed.
   */
  protected dispose() {
    // Dispose of the current webview panel
    if (this.panel) {
      this.panel.dispose();

      // Dispose of all disposables (i.e. commands) for the current webview panel
      while (this.disposables.length) {
        const disposable = this.disposables.pop();
        if (disposable) {
          disposable.dispose();
        }
      }
    }

    console.log("BasePanel.dispose() called");
  }

  /**
   * Defines and returns the HTML that should be rendered within the webview panel.
   *
   * @remarks This is also the place where references to the Svelte webview build files
   * are created and inserted into the webview HTML.
   *
   * @param webview A reference to the extension webview
   * @param extensionUri The URI of the directory containing the extension
   * @returns A template string literal containing the HTML that should be
   * rendered within the webview panel
   */
  private _getWebviewContent() {
    if (!this.panel) {
      // Should never happen, but just in case... show an error in the panel so the user can see (and report).
      return "Error: Missing webview panel instance";
    }
    // The CSS file from the Svelte build output
    const stylesUri = getUri(this.panel.webview, this.extensionUri, [
      "webview-ui",
      "public",
      "build",
      "bundle.css",
    ]);

    // The JS file from the Svelte build output
    const scriptUri = getUri(this.panel.webview, this.extensionUri, [
      "webview-ui",
      "public",
      "build",
      "bundle.js",
    ]);

    // The icon library being used.
    const iconsUri = getUri(this.panel.webview, this.extensionUri, [
      "webview-ui",
      "node_modules",
      "@vscode/codicons",
      "dist",
      "codicon.css",
    ]);

    const nonce = getNonce();

    // Origin for Content security policy source.
    const cspSource = this.panel.webview.cspSource;

    // Tip: Install the es6-string-html VS Code extension to enable code highlighting below

    return /*html*/ `
      <!DOCTYPE html>
      <html lang="en">
        <head>
          <title>Hello World</title>
          <meta charset="UTF-8" />
          <meta name="viewport" content="width=device-width, initial-scale=1.0" />
          <meta http-equiv="Content-Security-Policy" content="default-src ${cspSource}; connect-src http://localhost:*; font-src ${cspSource}; style-src ${cspSource} 'unsafe-inline'; img-src ${cspSource}; script-src ${cspSource} 'strict-dynamic' 'nonce-${nonce}'">      
          
          <link rel="stylesheet" type="text/css" href="${stylesUri}">
          <link rel="stylesheet" type="text/css" href="${iconsUri}" />      

          <script nonce="${nonce}">
            var suibase_panel_key = '${this.panelKey}';
          </script>
          <script defer nonce="${nonce}" src="${scriptUri}"></script>
        </head>
        <body>
        </body>
      </html>
    `;
  }

  /**
   * Sets up an event listener to listen for messages passed from the webview context and
   * executes code based on the message that is received.
   *
   * @param webview A reference to the extension webview
   * @param context A reference to the extension context
   */
  private _setWebviewMessageListener() {
    if (!this.panel) {
      console.log("Error: webview panel instance missing");
      return;
    }

    this.panel.webview.onDidReceiveMessage(
      (message: any) => {
        const command = message.command;
        const text = message.text;

        switch (command) {
          case "hello":
            // Code that should run in response to the hello message command
            window.showInformationMessage(text);
            return;
          // Add more switch case statements here as more webview message commands
          // are created within the webview context (i.e. inside media/main.js)
        }
      },
      undefined,
      this.disposables
    );
  }
}
