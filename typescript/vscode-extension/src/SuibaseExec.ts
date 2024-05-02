// An API that encapsulate interacting with the Suibase installation.
//
// A call may perform:
//  - Suibase CLI calls (e.g. "lsui", "localnet" etc...)
//  - JSON-RPC into Suibase
//
// All retrieved data is stored in a key/value cache.
//
// All cached value have a convenient unique "id" for
// versioning.

import * as vscode from "vscode";
import * as cp from "child_process";
import WebSocket from "ws";

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
  private static instance?: SuibaseExec;
  private static context?: vscode.ExtensionContext;
  private ws: WebSocket | undefined;

  // Define a cache to store the response
  private cache: any = {};

  private constructor() {
    // Should be called only by SuibaseExec.activate()
    this.ws = new WebSocket("ws://localhost:44399");

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
      this.cache.response = response.result;
      console.log(response.result);
    });
  }

  private dispose() {
    // Should be called only by SuibaseExec.deactivate()
    // Dispose of the instance resources (somewhat like a "destructor").
    if (this.ws) {
      this.ws.close();
      delete this.ws;
      this.ws = undefined;
    }
  }

  public static activate(context: vscode.ExtensionContext) {
    if (SuibaseExec.context) {
      console.log("Error: SuibaseExec.activate() called more than once");
      return;
    }

    SuibaseExec.context = context;
    SuibaseExec.instance = new SuibaseExec();
  }

  public static deactivate() {
    if (SuibaseExec.instance) {
      SuibaseExec.instance.dispose();
      delete SuibaseExec.instance;
      SuibaseExec.instance = undefined;
    } else {
      console.log("Error: SuibaseExec.deactivate() called out of order");
    }

    SuibaseExec.context = undefined;
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

  private async startDaemon() {
    // Check if suibase-daemon is running, if not, attempt
    // to start it and return once confirmed ready to
    // process requests.
    let suibaseRunning = false;
    try {
      const result = await execShell("lsof /tmp/.suibase/suibase-daemon.lock");
      // Check if "suibase" can be found in first column of one of the outputted line"
      const lines = result.split("\n");
      for (let i = 0; i < lines.length && !suibaseRunning; i++) {
        const line = lines[i];
        const columns = line.split(" ");
        if (columns[0].startsWith("suibase")) {
          suibaseRunning = true;
        }
      }
    } catch (err) {
      /* Do nothing */
    }

    if (!suibaseRunning) {
      // Start suibase daemon
      await execShell("~/suibase/scripts/common/run-daemon.sh suibase &");

      // TODO Implement retry and error handling of run-daemon.sh for faster startup.

      // Sleep 500 milliseconds to give it a chance to start.
      await new Promise((r) => setTimeout(r, 500));

      // TODO Confirm that suibase-daemon is responding to requests.
    }
  }

  private makeJsonRpcCall() {
    // Send a JSON-RPC request and handle its response.
    //
    // A valid response updates the cache if
    // a most recent data_version.
    //
    // On failure, keeps retrying until timeout.
    //
    // The caller get the response (or last known state) with
    // a lookup for the data in the cache.
    //
    const sb = SuibaseExec.getInstance();
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
