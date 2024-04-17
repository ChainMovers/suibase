import * as vscode from "vscode";
import { Disposable, Webview, WebviewPanel, window, Uri, ViewColumn } from "vscode";
import { getUri } from "../utilities/getUri";
import { getNonce } from "../utilities/getNonce";

/**
 * This class provides common functionality for all Suibase panel webview.
 *
 * Responsibilities include:
 * - Creating and rendering the webview.
 * - Setting the HTML, CSS and Typescript content of the webview.
 * - Properly cleaning up and disposing of webview resources when closed
 * - Setting message listeners so data can be passed between the webview and extension
 */
export class BaseWebview implements vscode.WebviewViewProvider {
  private static context?: vscode.ExtensionContext;

  // Instance variables initialized in constructor()
  private readonly key: string;
  private readonly title: string;
  private readonly extensionUri: Uri;
  private readonly extensionUris: Uri[] = [];

  // Instance variables initialized on first render()
  private webview: Webview | undefined;
  private panel: WebviewPanel | undefined;
  private disposables: Disposable[] = [];

  // Allow the subclasses read-access to the panel variable.
  protected getWebview() {
    if (!this.panel) {
      return this.webview;
    }
    return this.panel.webview;
  }

  protected getPanel() {
    return this.panel;
  }

  // Every view in the extension should have a unique key.
  //
  // This key is used to identify the view when the html is rendered.
  protected getKey() {
    return this.key;
  }

  protected getTitle() {
    return this.title;
  }

  /*
  protected getExtensionUri() {
    return this.extensionUri;
  }*/

  /**
   * The BaseWebview class private constructor (called only from the derived class).
   */
  protected constructor(key: string, title: string) {
    if (!BaseWebview.context) {
      console.log("Error: BaseWebview.constructor called before activate()");
      this.extensionUri = Uri.parse("file:///undefined");
    } else {
      this.extensionUri = BaseWebview.context.extensionUri;
      this.extensionUris = [
        Uri.joinPath(BaseWebview.context.extensionUri, "out"),
        Uri.joinPath(BaseWebview.context.extensionUri, "webview-ui/build"),
        Uri.joinPath(BaseWebview.context.extensionUri, "webview-ui/node_modules/@vscode/codicons/dist"),
      ];
    }

    this.key = key;
    this.title = title;
  }

  public static activate(context: vscode.ExtensionContext) {
    BaseWebview.context = context;
  }

  public static deactivate() {
    BaseWebview.context = undefined;
  }

  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this.webview = webviewView.webview;

    webviewView.webview.options = {
      // Allow scripts in the webview
      enableScripts: true,

      localResourceRoots: this.extensionUris,
    };

    webviewView.webview.html = this._getWebviewContent();

    // Register message handling.
    webviewView.webview.onDidReceiveMessage(
      (message: any) => {
        console.log("BaseWebview.onDidReceiveMessage() called");
        this.handleMessage(message);
      },
      undefined,
      this.disposables
    );
  }

  /**
   * Renders the current webview panel if it exists otherwise a new webview panel
   * will be created and displayed.
   *
   * @param extensionUri The URI of the directory containing the extension.
   * @param panelKey A unique string that identifies the webview panel.
   * @param panelTitle The title shown for this webview panel.
   */
  protected renderPanel() {
    // Look if there is already a BasePanel instance for the given key.
    if (this.panel !== undefined) {
      // If the webview panel already exists reveal it
      this.panel.reveal(ViewColumn.One);
      console.log("BaseWebview render_panel reveal() called");
    } else {
      // If a webview panel does not already exist create and show a new one
      // "this" here is the subclass that extends BasePanel.
      this.panel = window.createWebviewPanel(
        this.key, // Panel view type, must match what is in package.json
        this.title,
        // The editor column the panel should be displayed in
        ViewColumn.One,
        // Extra panel configurations
        {
          // Enable JavaScript in the webview
          enableScripts: true,
          // Restrict the webview to only load resources from the `out` and `webview-ui/build` directories
          localResourceRoots: this.extensionUris,
        }
      );

      // Set an event listener to listen for when the panel is disposed (i.e. when the user closes
      // the panel or when the panel is closed programmatically)
      this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

      // Set the HTML content for the webview panel
      this.panel.webview.html = this._getWebviewContent();

      // Register message handling.
      this.panel.webview.onDidReceiveMessage(
        (message: any) => {
          this.handleMessage(message);
        },
        undefined,
        this.disposables
      );
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

    console.log("BaseWebview.dispose() called");
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
    let webview = this.getWebview();
    if (!webview) {
      // Should never happen, but just in case... show an error so the user can see (and report).
      return "Error: Missing webview instance";
    }

    // The CSS file from the Svelte build output
    const stylesUri = getUri(webview, this.extensionUri, ["webview-ui", "build", "assets", "index.css"]);

    // The JS file from the Svelte build output
    const scriptUri = getUri(webview, this.extensionUri, ["webview-ui", "build", "assets", "index.js"]);

    // The icon library being used.
    const iconsUri = getUri(webview, this.extensionUri, [
      "webview-ui",
      "node_modules",
      "@vscode/codicons",
      "dist",
      "codicon.css",
    ]);

    const nonce = getNonce();

    // Origin for Content security policy source.
    const cspSource = webview.cspSource;

    // Tip: Install the es6-string-html VS Code extension to enable code highlighting below

    return /*html*/ `
      <!DOCTYPE html>
      <html lang="en">
        <head>
          <title>Webview-UI</title>
          <meta charset="UTF-8" />
          <meta name="viewport" content="width=device-width, initial-scale=1.0" />
          <meta http-equiv="Content-Security-Policy" content="default-src ${cspSource}; connect-src http://localhost:*; font-src ${cspSource}; style-src ${cspSource} 'unsafe-inline'; img-src ${cspSource}; script-src ${cspSource} 'strict-dynamic' 'nonce-${nonce}'">      
          
          <link rel="stylesheet" type="text/css" href="${stylesUri}">
          <link rel="stylesheet" type="text/css" href="${iconsUri}" />
        </head>
        <body>
          <script nonce="${nonce}">
            var suibase_view_key = '${this.key}';
          </script>
          <div id="root"></div>
          <script type="module" nonce="${nonce}" src="${scriptUri}"></script>
        </body>
      </html>
    `;
  }

  protected handleMessage(message: any): void {
    // This is a placeholder for the derived class to implement.
    // The derived class should override this method to handle messages
    // sent from the webview context.
  }

  protected postMessage(message: any): void {
    if (this.panel) {
      this.panel.webview.postMessage(message);
    } else if (this.webview) {
      this.webview.postMessage(message);
    } else {
      console.log("Warning: postMessage() called without a valid panel or webview");
    }
  }
}
