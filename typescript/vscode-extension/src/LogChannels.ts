// Control output channels for logging.
//
// "Sui Console (Active)"  <-- Display for the active workdir. Shows a "switch message" when changing the active workdir.
//
// Others display only messages specific to a workdir:
// "Sui Console (Localnet)"
// "Sui Console (Devnet)"
// "Sui Console (Testnet)"
// "Sui Console (Mainnet)"

import { WORKDIRS_KEYS } from "./common/Consts";

import { window, OutputChannel } from "vscode";

export class LogChannels {
  // LogChannels is a singleton.
  private static instance?: LogChannels;
  private activeWorkdir;
  private outputWorkdirs: Record<string, OutputChannel> = {};
  private outputActive: OutputChannel;

  private constructor() {
    this.activeWorkdir = ""; // WORKDIRS_KEYS[WORKDIR_IDX_TESTNET]
    this.outputActive = window.createOutputChannel("Sui Console (active)", { log: true });
    for (const key of WORKDIRS_KEYS) {
      const name = `Sui Console (${key})`;

      this.outputWorkdirs[key] = window.createOutputChannel(name, { log: true });
    }
    this.outputActive.show();
  }

  public setActiveWorkdir(workdir: string) {
    // Find the index of the workdir in WORKDIRS_KEYS
    const idx = WORKDIRS_KEYS.indexOf(workdir);
    if (idx < 0) {
      console.error(`Error: setActiveWorkdir(${workdir}) called with invalid workdir`);
      return;
    }

    if (this.activeWorkdir !== workdir) {
      this.activeWorkdir = workdir;
      this.outputActive.appendLine(`***Switching to console.log from ${workdir}****`);
    }
  }

  public static getInstance(): LogChannels {
    if (!LogChannels.instance) {
      LogChannels.instance = new LogChannels();
    }
    return LogChannels.instance;
  }

  public static activate() {
    //console.log("SuibaseData.activateForExtension() called");
    if (LogChannels.instance) {
      console.log("Error: LogChannels.activate() called more than once");
      return;
    }

    LogChannels.getInstance(); // Create the singleton
  }

  public static deactivate() {
    if (LogChannels.instance) {
      delete LogChannels.instance;
      LogChannels.instance = undefined;
    } else {
      console.log("Error: LogChannels.deactivate() called out of order");
    }
  }
}
