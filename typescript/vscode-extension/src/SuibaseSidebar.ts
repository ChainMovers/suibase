import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

// References:
//  https://code.visualstudio.com/api/extension-guides/tree-view
//  https://github.com/microsoft/vscode-extension-samples/tree/main/tree-view-sample
export class DepNodeProvider implements vscode.TreeDataProvider<Dependency> {
  private _onDidChangeTreeData: vscode.EventEmitter<Dependency | undefined | void> = new vscode.EventEmitter<
    Dependency | undefined | void
  >();
  readonly onDidChangeTreeData: vscode.Event<Dependency | undefined | void> = this._onDidChangeTreeData.event;

  constructor(private workdir: string, private workspaceRoot: string | undefined) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: Dependency): vscode.TreeItem {
    return element;
  }

  getChildren(element?: Dependency): Thenable<Dependency[]> {
    /*
		if (!this.workspaceRoot) {
			vscode.window.showInformationMessage('No dependency in empty workspace');
			return Promise.resolve([]);
		}

		if (element) {
			return Promise.resolve(this.getDepsInPackageJson(path.join(this.workspaceRoot, 'node_modules', element.label, 'package.json')));
		} else {
			const packageJsonPath = path.join(this.workspaceRoot, 'package.json');
			if (this.pathExists(packageJsonPath)) {
				return Promise.resolve(this.getDepsInPackageJson(packageJsonPath));
			} else {
				vscode.window.showInformationMessage('Workspace has no package.json');
				return Promise.resolve([]);
			}
		}*/
    if (element) {
      return Promise.resolve(this.getDepsInPackageJson(this.workdir));
    } else {
      return Promise.resolve(this.getDepsInPackageJson("NULL"));
    }
  }

  /**
   * Given the path to package.json, read all its dependencies and devDependencies.
   */
  private getDepsInPackageJson(packageJsonPath: string): Dependency[] {
    const toDep = (moduleName: string, parentName: string): Dependency => {
      if (parentName === "NULL") {
        return new Dependency(moduleName, parentName, vscode.TreeItemCollapsibleState.Collapsed);
      } else {
        return new Dependency(moduleName, parentName, vscode.TreeItemCollapsibleState.None);
      }
    };
    const deps = [
      toDep("Localnet", packageJsonPath),
      toDep("Devnet", packageJsonPath),
      toDep("Testnet", packageJsonPath),
      toDep("Mainnet", packageJsonPath),
    ];
    return deps;
    /*
		const workspaceRoot = this.workspaceRoot;
		if (this.pathExists(packageJsonPath) && workspaceRoot) {
			const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));


			const deps = packageJson.dependencies
				? Object.keys(packageJson.dependencies).map(dep => toDep(dep, packageJson.dependencies[dep]))
				: [];
			const devDeps = packageJson.devDependencies
				? Object.keys(packageJson.devDependencies).map(dep => toDep(dep, packageJson.devDependencies[dep]))
				: [];
			return deps.concat(devDeps);
		} else {
			return [];
		}
    */
  }

  private pathExists(p: string): boolean {
    try {
      fs.accessSync(p);
    } catch (err) {
      return false;
    }

    return true;
  }
}

export class Dependency extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    private readonly version: string,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState,
    public readonly command?: vscode.Command
  ) {
    super(label, collapsibleState);

    this.tooltip = `${this.label}-${this.version}`;
    this.description = this.version;
  }

  iconPath = {
    light: path.join(__filename, "..", "..", "resources", "light", "dependency.svg"),
    dark: path.join(__filename, "..", "..", "resources", "dark", "dependency.svg"),
  };

  contextValue = "dependency";
}

export class SuibaseSidebar {
  private static instance?: SuibaseSidebar;
  private static context?: vscode.ExtensionContext;

  private constructor() {} // Called only from activate().
  private dispose() {} // Called only from deactivate().

  public static activate(context: vscode.ExtensionContext) {
    if (SuibaseSidebar.instance) {
      console.log("Error: SuibaseSidebar.activate() called more than once");
      return;
    }

    SuibaseSidebar.context = context;
    SuibaseSidebar.instance = new SuibaseSidebar();

    // Registration of the tree view.
    // Code for the tree view
    const rootPath =
      vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0
        ? vscode.workspace.workspaceFolders[0].uri.fsPath
        : undefined;

    // Do createTreeView if rootPath is defined
    {
      const tree = new DepNodeProvider("suibase", rootPath);
      vscode.window.registerTreeDataProvider("suibaseTreeView", tree);
    }
  }

  public static deactivate() {
    if (SuibaseSidebar.instance) {
      SuibaseSidebar.instance.dispose();
      delete SuibaseSidebar.instance;
      SuibaseSidebar.instance = undefined;
    } else {
      console.log("Error: SuibaseSidebar.deactivate() called out of order");
    }

    SuibaseSidebar.context = undefined;
  }

  public static getInstance(): SuibaseSidebar | undefined {
    if (!SuibaseSidebar.instance) {
      console.log("Error: SuibaseSidebar.getInstance() called before activate()");
    }
    return SuibaseSidebar.instance;
  }
}
