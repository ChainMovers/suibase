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
import * as OS from "os";
import * as fs from "fs/promises";

// Execute the shell command and return its output. This is a blocking call.
const execShell = (cmd: string) =>
  new Promise<string>((resolve, reject) => {
    cp.exec(cmd, (err, out) => {
      if (err) {
        return reject(err);
      }
      return resolve(out);
    });
  });

// Execute the shell command in the background.
//
// This is a non-blocking call.
//
// It is assumed the caller does not depend on the command output
// to verify its success.
const execShellBackground = (cmd: string): Promise<void> =>
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  new Promise<void>((resolve) => {
    cp.exec(cmd, (err, stdout, stderr) => {
      if (err) {
        console.warn(err, `${stdout}${stderr}`);
      }
      resolve();
    });
  });

export class SuibaseExec {
  private static instance?: SuibaseExec;
  private static context?: vscode.ExtensionContext;
  private static homedir: string;
  //private ws: WebSocket | undefined;

  // Define a cache to store the response
  //private cache: any = {};

  private constructor() {
    SuibaseExec.homedir = OS.homedir();

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

  public static getHomedir(): string {
    return SuibaseExec.homedir;
  }

  public async isRustInstalled(): Promise<boolean> {
    // Returns true if the rust compiler can be call.
    // Returns false on any error.
    try {
      const result = await execShell("rustc --version");
      if (result.startsWith("rustc") && !result.includes("error")) {
        return true;
      }
    } catch (error: any) {
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

  public canonicalPath(pathname: string): string {
    // Expand the pathname to its absolute path.
    // This function must always resolve its promise.
    if (pathname.startsWith("~")) {
      pathname = pathname.replace(/^~/, SuibaseExec.homedir);
    }
    return pathname;
  }

  public async fileExists(pathname: string): Promise<boolean> {
    // Returns true if the file exists on the filesystem.
    // Returns false on any error.
    //
    // This function must always resolve its promise.
    try {
      // Attempt to access the file
      await fs.access(this.canonicalPath(pathname));
      return true; // If no error is thrown, the file exists
    } catch (error: any) {
      // console.error(`File ${pathname} not found:`, error.message);
      return false; // If an error is thrown, the file does not exist
    }
  }

  public async isSuibaseInstalled(): Promise<boolean> {
    // Verify if Suibase itself is installed.
    //
    // Good enough to just check that the script to repair exists
    // (it is used for recovery attempt by the extension).
    try {
      return await this.fileExists("~/suibase/repair");
    } catch (error) {
      return false;
    }
  }

  public async isSuibaseOnPath(): Promise<boolean> {
    // Verify if Suibase itself is accessible.
    //
    // Verifies that executing "localnet suibase-script-name" returns the exact string "localnet".
    //
    // This is similar to the verification trick used by "~/suibase/install"
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
      const result = await execShell("lsof /tmp/.suibase/suibase-daemon.lock 2>/dev/null");
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
      return await this.fileExists("/tmp/.suibase/suibase-daemon-upgrading");
    } catch (error) {
      return false;
    }
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
        const pathname = this.canonicalPath("~/suibase/repair");
        void execShellBackground(`${pathname}`);

        // Check for up to ~60 seconds that it is started.
        let attempts = 120;
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
