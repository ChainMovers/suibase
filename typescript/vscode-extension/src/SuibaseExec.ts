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
//import WebSocket from "ws";

const execShell = (cmd: string) =>
  new Promise<string>((resolve, reject) => {
    cp.exec(cmd, (err, out) => {
      if (err) {
        return reject(err);
      }
      return resolve(out);
    });
  });

const execShellBackground = (cmd: string) =>
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  new Promise<string>((resolve, _reject) => {
    cp.exec(cmd, (err, stdout, stderr) => {
      if (err) {
        console.warn(err);
      }
      resolve(stdout ? stdout : stderr);
    });
  });

export class SuibaseExec {
  private static instance?: SuibaseExec;
  private static context?: vscode.ExtensionContext;
  //private ws: WebSocket | undefined;

  // Define a cache to store the response
  //private cache: any = {};

  private constructor() {
    // Should be called only by SuibaseExec.activate()
    /*
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
    });*/
  }

  private dispose() {
    // Should be called only by SuibaseExec.deactivate()
    // Dispose of the instance resources (somewhat like a "destructor").
    /*
    if (this.ws) {
      this.ws.close();
      delete this.ws;
      this.ws = undefined;
    }*/
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

  public async isRustInstalled(): Promise<boolean> {
    // Returns true if the rust compiler can be call.
    // Returns false on any error.
    try {
      const result = await execShell("rustc --version");
      if (result.startsWith("rustc") && !result.includes("error")) {
        return true;
      }
    } catch (error) {
      console.error("rustc not installed");
    }
    return false;
  }

  public async isGitInstalled(): Promise<boolean> {
    // Returns true if the git can be call.
    // Returns false on any error.
    try {
      const result = await execShell("git --version");
      if (result.startsWith("git") && !result.includes("error")) {
        return true;
      }
    } catch (error) {
      console.error("git not installed");
    }
    return false;
  }

  /*
  public async fileExists(pathname: string): Promise<boolean> {
    // Returns true if the file exists on the filesystem.
    // Returns false on any error.
    //
    // This function must always resolve its promise.
    try {
      let result = await execShell(`ls ${pathname}`);
      result = result.toLowerCase();
      if (!result.includes(pathname) || result.includes("cannot access") || result.includes("no such")) {
        return false;
      }
    } catch (error) {
      return false;
    }
    return true;
  }*/

  public async isSuibaseInstalled(): Promise<boolean> {
    // Verify if Suibase itself is installed.
    //
    // Verifies that executing "localnet suibase-script-name" returns the exact string "localnet".
    //
    // This is the same verification trick used by "~/suibase/install"
    try {
      const result = await execShell("localnet suibase-script-name");
      if (!result || result.trim() !== "localnet") {
        return false;
      }
    } catch (error) {
      return false;
    }

    return true;
  }

  public async isSuibaseBackendRunning(): Promise<boolean> {
    // Returns true if suibase-daemon is running.
    // This function must always resolve its promise.
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
      return false;
    }
    return suibaseRunning;
  }

  public async isSuibaseBackendUpgrading(): Promise<boolean> {
    // True when a file /tmp/suibase_daemon_upgrading exists.
    //
    // This file exists only while a backend scripts is in the process of
    // upgrading the daemon.
    //
    // This function must always resolve its promise.
    try {
      let result = await execShell("ls /tmp/.suibase/suibase-daemon-upgrading");
      result = result.toLowerCase();
      if (result.includes("cannot") || result.includes("no such")) {
        return false;
      }
    } catch (error) {
      return false;
    }

    return true;
  }

  public async version(): Promise<string> {
    // This function must always resolve its promise.
    // Returns an empty string on error.
    try {
      const result = await execShell("localnet --version");
      //console.log(result);
      return result;
    } catch (err) {
      /* Absorb the err and return empty string */
    }

    return "";
  }

  public async startDaemon(): Promise<boolean> {
    // Check if suibase-daemon is running, if not, attempt
    // to start it and return once confirmed ready to
    // process requests.
    //
    // This function must always resolve its promise.
    try {
      let suibaseRunning = await this.isSuibaseBackendRunning();

      if (!suibaseRunning) {
        // Start suibase daemon
        void execShellBackground("~/suibase/scripts/common/run-daemon.sh suibase");

        // Check for up to ~5 seconds that it is started.
        let attempts = 10;
        while (!suibaseRunning && attempts > 0) {
          // Sleep 500 millisecs to give it a chance to start.
          await new Promise((r) => setTimeout(r, 500));
          suibaseRunning = await this.isSuibaseBackendRunning();
          attempts--;
        }
      }

      if (suibaseRunning) {
        return true;
      }
    } catch (err) {
      /* Absorb the err and return false below */
    }
    console.error("Failed to start suibase.daemon");
    return false;
  }
  /*
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
  */
}
