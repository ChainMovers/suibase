// This is a custom Hook used by all webview.
//
// Maintains latest JSON data from the backend and translate it to React states.
//
// Tracking is done periodically (and reactively) with the JSON response from getVersions
// and then trig further JSON retrieval as needed (e.g getWorkdirStatus).
//
// Why all that complexity?
//    With VSCode, the suibase backend daemon and the webview(s) are not always running
//    on the same machine (e.g. user does remote debugging).
//
//    The extension code is acting as the middlemen to forward all the needed data
//    using "Post" messages.

import { useRef, useState, useEffect } from "react";
import {
  SuibaseJsonVersions,
  SuibaseJsonWorkdirPackages,
  SuibaseJsonWorkdirStatus,
} from "../common/SuibaseJson";
import { WORKDIRS_KEYS } from "../common/Consts";
import { VSCode } from "../lib/VSCode";
import { InitView, RequestWorkdirPackages, RequestWorkdirStatus } from "../common/ViewMessages";

export class ViewWorkdirData {
  public label: string;
  public workdir: string;
  public workdirIdx: number;
  public versions: SuibaseJsonVersions; // Backend JSON of getVersions for this Workdir.
  public workdirStatus: SuibaseJsonWorkdirStatus; // Backend JSON of getWorkdirStatus for this Workdir.
  public workdirPackages: SuibaseJsonWorkdirPackages; // Backend JSON of getWorkdirPackages for this Workdir.

  constructor(workdir: string, workdirIdx: number) {
    this.label = workdir.charAt(0).toUpperCase() + workdir.slice(1);
    this.workdir = workdir;
    this.workdirIdx = workdirIdx;
    this.versions = new SuibaseJsonVersions();
    this.workdirStatus = new SuibaseJsonWorkdirStatus();
    this.workdirPackages = new SuibaseJsonWorkdirPackages();
  }
}

export class ViewCommonData {
  private _activeWorkdir: string;
  private _activeWorkdirIdx: number;
  private _activeLoaded: boolean;
  private _setupIssue: string;

  constructor() {
    this._activeWorkdir = "";
    this._activeWorkdirIdx = 0;
    this._activeLoaded = false;
    this._setupIssue = "";
  }

  public get activeWorkdir() {
    return this._activeWorkdir;
  }

  public get activeWorkdirIdx() {
    return this._activeWorkdirIdx;
  }

  public get activeLoaded() {
    return this._activeLoaded;
  }

  public get setupIssue() {
    return this._setupIssue;
  }

  public set activeWorkdir(workdir: string) {
    const idx = WORKDIRS_KEYS.indexOf(workdir);
    if (idx < 0) {
      console.error(`Invalid workdir: ${workdir}`);
      return;
    }
    // Keep activeWorkdirIdx in-sync.
    this._activeWorkdirIdx = idx;
    this._activeWorkdir = workdir;
    this._activeLoaded = true;
  }

  public set activeWorkdirIdx(workdirIdx: number) {
    // Check that workdirIdx is in-range.
    if (workdirIdx < 0 || workdirIdx >= WORKDIRS_KEYS.length) {
      console.error(`Invalid workdirIdx: ${workdirIdx}`);
      return;
    }
    this._activeWorkdir = WORKDIRS_KEYS[workdirIdx];
    this._activeWorkdirIdx = workdirIdx;
    this._activeLoaded = true;
  }

  public set setupIssue(issue: string) {
    this._setupIssue = issue;
  }
}

interface CommonControllerOptions {
  trackStatus?: boolean;
  trackPackages?: boolean;
}

export const useCommonController = (sender: string, options?: CommonControllerOptions) => {
  const trackStatus = options?.trackStatus || false;
  const trackPackages = options?.trackPackages || false;

  const common = useRef(new ViewCommonData());
  const [workdirs] = useState<ViewWorkdirData[]>(
    WORKDIRS_KEYS.map((key, index) => new ViewWorkdirData(key, index))
  );

  const log_debug = false; // sender ? sender == WEBVIEW_DASHBOARD : false;

  // A state where the view should display a string to the user while there is a backend issue.
  //const [setupIssue, setSetupIssue] = useState("");

  // States to force re-renders for ALL components using useCommonController.
  // useEffects dependencies on these can be used for more selectively
  // reacting to changes.
  const [commonTrigger, setUpdateTrigger] = useState(0);
  const [statusTrigger, setStatusTrigger] = useState(0);
  const [packagesTrigger, setPackagesTrigger] = useState(0);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      if (event.data) {
        processMessage(event.data);
      }
    };
    window.addEventListener("message", handleMessage);
    return () => window.removeEventListener("message", handleMessage);
  }, []);

  useEffect(() => {
    // This is also called when this component is mounted.

    // Call InitView if any of the backend data is missing.
    // TODO Use persistence to cache the data when the view is unmounted.
    let missingData = common.current.activeLoaded === false;
    if (!missingData) {
      // Check workdirs data.
      for (let i = 0; i < workdirs.length; i++) {
        const workdirTracking = workdirs[i];
        if (!workdirTracking.versions.getJson()) {
          missingData = true;
          break;
        }
      }
    }
    if (missingData) {
      log_debug && console.log("Calling InitView");
      VSCode.postMessage(new InitView(sender));
    }
  }, [workdirs]);
  const processMessage = (message: any) => {
    try {
      if (message.name) {
        let do_common_trigger = false;
        let do_status_trigger = false;
        let do_packages_trigger = false;
        switch (message.name) {
          case "UpdateVersions": {
            // Detect when the Backend is not responding.
            log_debug && console.log("UpdateVersions", message);
            let backendIssue = false;
            if (message.setupIssue) {
              const msgSetupIssue = message.setupIssue as string;
              if (msgSetupIssue !== common.current.setupIssue) {
                common.current.setupIssue = msgSetupIssue;
                do_common_trigger = true;
              }
              if (msgSetupIssue !== "") {
                backendIssue = true;
              }
            }

            if (backendIssue === false && common.current.setupIssue !== "") {
              common.current.setupIssue = "";
              do_common_trigger = true;
            }

            if (backendIssue === false && message.json) {
              const workdirTracking = workdirs[message.workdirIdx];
              log_debug &&
                console.log(
                  "workdir ",
                  message.workdirIdx,
                  "this.json",
                  workdirTracking.versions.getJson(),
                  "this.json === null",
                  workdirTracking.versions.getJson() === null
                );

              const hasChanged = workdirTracking.versions.update(message.json);
              if (!hasChanged) {
                log_debug &&
                  console.log("workdir ", message.workdirIdx, "UpdateVersions NOT changed", message.json);
              } else {
                log_debug &&
                  console.log("workdir ", message.workdirIdx, "UpdateVersions changed", message.json);
                do_common_trigger = true;
                // Verify if versions shows that a newer WorkdirStatus is available. If yes, then PostMessage "RequestWorkdirStatus"
                if (trackStatus) {
                  const [isUpdateNeeded, methodUuid, dataUuid] =
                    workdirTracking.versions.isWorkdirStatusUpdateNeeded(workdirTracking.workdirStatus);

                  if (isUpdateNeeded) {
                    log_debug &&
                      console.log(
                        "workdir ",
                        message.workdirIdx,
                        "UpdateVersion update needed request posted"
                      );
                    VSCode.postMessage(
                      new RequestWorkdirStatus(sender, message.workdirIdx, methodUuid, dataUuid)
                    );
                  }
                }

                // Do same for WorkdirPackages.
                if (trackPackages) {
                  const [isUpdateNeeded, methodUuid, dataUuid] =
                    workdirTracking.versions.isWorkdirPackagesUpdateNeeded(workdirTracking.workdirPackages);

                  if (isUpdateNeeded) {
                    VSCode.postMessage(
                      new RequestWorkdirPackages(sender, message.workdirIdx, methodUuid, dataUuid)
                    );
                  }
                }
              }
              // As needed, update activeWorkdir (and indirectly activeWorkdirIdx ).
              //console.log(`common.current.activeWorkdir: ${common.current.activeWorkdir}, message.json.asuiSelection: ${message.json.asuiSelection}`);
              if (common.current.activeWorkdir !== message.json.asuiSelection) {
                common.current.activeWorkdir = message.json.asuiSelection;
                do_common_trigger = true;
              }
            }
            break;
          }

          case "UpdateWorkdirStatus": {
            if (trackStatus) {
              const workdirTracking = workdirs[message.workdirIdx];
              const hasChanged = workdirTracking.workdirStatus.update(message.json);
              if (hasChanged) {
                do_status_trigger = true;
              }
            }
            break;
          }

          case "UpdateWorkdirPackages": {
            if (trackPackages) {
              const workdirTracking = workdirs[message.workdirIdx];
              const hasChanged = workdirTracking.workdirPackages.update(message.json);
              if (hasChanged) {
                do_packages_trigger = true;
              }
            }
            break;
          }

          default:
            console.log("Received an unknown command", message);
        }

        if (do_common_trigger) {
          setUpdateTrigger((prev) => prev + 1);
        }
        if (do_status_trigger) {
          setStatusTrigger((prev) => prev + 1);
        }
        if (do_packages_trigger) {
          setPackagesTrigger((prev) => prev + 1);
        }
      }
    } catch (error) {
      console.error("An error occurred in useCommonController:", error);
    }
  };

  // Note: Triggers are intended as "finer grain" dependencies for useEffects.
  //       Also, it makes possible reaction on changes *within* objects/arrays.
  return { commonTrigger, statusTrigger, packagesTrigger, common, workdirs };
};
