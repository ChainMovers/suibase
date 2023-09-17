// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';

import { DepNodeProvider, Dependency } from './suibaseSidebar';

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {

	// Use the console to output diagnostic information (console.log) and errors (console.error)
	// This line of code will only be executed once when your extension is activated
	console.log('Congratulations, your extension "suibase" is now active!');

	// The command has been defined in the package.json file
	// Now provide the implementation of the command with registerCommand
	// The commandId parameter must match the command field in package.json
	let disposable = vscode.commands.registerCommand('suibase.suibase', () => {
		// The code you place here will be executed every time your command is executed
		// Display a message box to the user
		vscode.window.showInformationMessage('Suibase running in localdev!');
	});

	context.subscriptions.push(disposable);
	// Code for the tree view
	const rootPath =
  vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0
    ? vscode.workspace.workspaceFolders[0].uri.fsPath
    : undefined;

	// Do createTreeView if rootPath is defined
	{
	  const tree = new DepNodeProvider("localnet",rootPath);
	  vscode.window.registerTreeDataProvider('localnetTreeView', tree);
	}

	{
		const tree = new DepNodeProvider("devnet",rootPath);
		vscode.window.registerTreeDataProvider('devnetTreeView', tree);
	}

	{
		const tree = new DepNodeProvider("testnet",rootPath);
		vscode.window.registerTreeDataProvider('testnetTreeView', tree);
	}

	{
		const tree = new DepNodeProvider("mainnet",rootPath);
		vscode.window.registerTreeDataProvider('mainnetTreeView', tree);
	}

}

// This method is called when your extension is deactivated
export function deactivate() {}