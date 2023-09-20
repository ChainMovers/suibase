// An API that encapsulate getting data from the Suibase installation.
//
// A call may perform:
//  - Suibase CLI calls (e.g. "lsui", "localnet" etc...)
//  - JSON-RPC into Suibase
//
// All retreived data is stored in a key/value cache.
//
// All cached value have a convenient unique "id" for
// versioning.

import * as vscode from "vscode";
import * as cp from "child_process";
import * as WebSocket from "ws";

const execShell = (cmd: string) =>
  new Promise<string>((resolve, reject) => {
    cp.exec(cmd, (err, out) => {
      if (err) {
        return reject(err);
      }
      return resolve(out);
    });
  });

export class SuibaseExec {
  private static instance: SuibaseExec | undefined;
  private static context: vscode.ExtensionContext | undefined;
  private ws: WebSocket | undefined;

  // Define a cache to store the response
  private cache: any = {};

  private constructor() {
    // Should be called only by SuibaseExec.activate()
    this.ws = new WebSocket("ws://0.0.0.0:44399");

    this.ws.on("open", () => {
      console.log("WebSocket connection opened");
    });

    this.ws.on("close", () => {
      console.log("WebSocket connection closed");
    });

    this.ws.on("message", (data: any) => {
      // Parse the response
      const response = JSON.parse(data);

      // Store the response in the cache
      //let self = SuibaseExec.getInstance();
      this.cache["response"] = response.result;
      console.log(response.result);
    });
  }

  private deactivate() {
    // Dispose of the instance (somewhat like a "destructor").
    // Should be called only by the public SuibaseExec.deactivate()
    if (this.ws) {
      this.ws.close();
      delete this.ws;
      this.ws = undefined;
    }
  }

  public static activate(context: vscode.ExtensionContext) {
    if (!typeof SuibaseExec.context === undefined) {
      console.log("Error: SuibaseExec.activate() called more than once");
      return;
    }

    SuibaseExec.context = context;
    SuibaseExec.instance = new SuibaseExec();
  }

  public static deactivate() {
    if (SuibaseExec.instance) {
      SuibaseExec.instance.deactivate();
      delete SuibaseExec.instance;
    }

    if (SuibaseExec.context) {
      SuibaseExec.context = undefined;
    } else {
      console.log("Error: SuibaseExec.deactivate() called more than once");
    }
  }

  public static getInstance(): SuibaseExec | undefined {
    if (!SuibaseExec.instance) {
      console.log("Error: SuibaseExec.getInstance() called before activate()");
    }
    return SuibaseExec.instance;
  }

  public async version(): Promise<string> {
    try {
      const result = await execShell("localnet --version");
      console.log(result);
      return Promise.resolve(result);
    } catch (err) {
      return Promise.reject(err);
    }
  }

  private makeJsonRpcCall() {
    // Best effort sending of the JSON-RPC request.
    // The eventual response will update the cache if
    // a most recent data_version.
    let sb = SuibaseExec.getInstance();
    if (!sb) {
      return;
    }
    if (sb.ws) {
      // Construct the JSON-RPC 2.0 request
      const request = {
        jsonrpc: "2.0",
        method: "getLinks",
        params: { workdir: "localnet" },
        id: 1,
      };

      // Send the request
      sb.ws.send(JSON.stringify(request));
    }
  }

  public async getLinks(): Promise<string> {
    try {
      // Do a JSON-RPC getLinks method call using ws
      this.makeJsonRpcCall();
      return Promise.resolve("sent");
    } catch (err) {
      return Promise.reject(err);
    }
  }
}
