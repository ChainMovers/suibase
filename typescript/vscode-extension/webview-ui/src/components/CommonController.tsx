// This is a custom Hook used by all other view/controller components.
//
// It keep tracks of the "versions" for each workdir and 
// "status" data for the active workdir.
import { useRef, useState, useEffect } from 'react';
import { useMessage } from "../lib/CustomHooks";
import { SuibaseJsonVersions, SuibaseJsonWorkdirStatus } from "../common/SuibaseJson";
import { WORKDIRS_KEYS } from "../common/Consts";
import { VSCode } from '../lib/VSCode';
import { InitView, RequestWorkdirStatus } from '../common/ViewMessages';

export class ViewWorkdirData {
    private _label: string;
    private _workdir: string;
    private _workdirIdx: number;
    private _suiClientVersionShort: string;
    private _versions: SuibaseJsonVersions; // Backend JSON of getVersions for this Workdir.
    private _workdirStatus: SuibaseJsonWorkdirStatus; // Backend JSON of getWorkdirStatus for this Workdir.

    constructor(workdir: string, workdirIdx: number) {
      this._label = workdir.charAt(0).toUpperCase() + workdir.slice(1);      
      this._workdir = workdir;
      this._workdirIdx = workdirIdx;
      this._suiClientVersionShort = "";
      this._versions = new SuibaseJsonVersions();
      this._workdirStatus = new SuibaseJsonWorkdirStatus();
    }

    public get label() {
      return this._label;
    }

    public get workdir() {
      return this._workdir;
    }

    public get workdirIdx() {
      return this._workdirIdx;
    }

    public get status() {
      return this._workdirStatus.status;
    }

    public get suiClientVersion() {
      return this._workdirStatus.suiClientVersion;
    }

    public get suiClientVersionShort() {
      return this._suiClientVersionShort;
    }

    public get versions() {
      return this._versions;
    }

    public get workdirStatus() {
      return this._workdirStatus;
    }

    public get isStatusLoaded() {
      return this._workdirStatus.isLoaded;
    }

    public updateCalculatedFields() {
      if (typeof this._workdirStatus.suiClientVersion === 'string' && this._workdirStatus.suiClientVersion.length > 0) {
        this._suiClientVersionShort = this._workdirStatus.suiClientVersion.split("-")[0];
      } else {
        this._suiClientVersionShort = "";
      }      
    }
  }

export class ViewCommonData {
    private _activeWorkdir: string;
    private _activeWorkdirIdx: number;
    private _activeLoaded: boolean;

    constructor() {
      this._activeWorkdir = "localnet";
      this._activeWorkdirIdx = 0;
      this._activeLoaded = false;
    }

    public get activeWorkdir() {
      return this._activeWorkdir;
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

    public get activeWorkdirIdx() {
      return this._activeWorkdirIdx;
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

    public get activeLoaded() {
      return this._activeLoaded;
    }
}

export const useCommonController = () => {
  const { message } = useMessage();
  const common = useRef(new ViewCommonData());
  const workdirs = useRef(WORKDIRS_KEYS.map((key, index) => new ViewWorkdirData(key, index)));
  const [, setUpdateTrigger] = useState(false);

  useEffect(() => {  
    // Called when this component is mounted, which is surprisingly often (e.g. every time user switch tabs in VSCode).

    // Call InitView if any of the backend data is missing, otherwise use the latest cached in ref.
    let missingData = common.current.activeLoaded === false;
    if (!missingData) {
      // Check workdirs data.
      for (let i = 0; i < workdirs.current.length; i++) {
        const workdirTracking = workdirs.current[i];
        if (!workdirTracking.versions.getJson()) {
          missingData = true;
          break;
        }
      }
    }
    if (missingData) {
      VSCode.postMessage(new InitView());
    }
  }, []);

  useEffect(() => {
    try {
      if (message && message.name) {
        let do_render = false;
        switch (message.name) {
          case 'UpdateVersions': {
            const workdirTracking = workdirs.current[message.workdirIdx];
            const hasChanged = workdirTracking.versions.update(message.json);            
            if (hasChanged) {
              do_render = true;
              // Verify if versions shows that a newer WorkdirStatus is available. If yes, then PostMessage "RequestWorkdirStatus"              
              //console.log(`Received modified versions ${JSON.stringify(message.json)} workdir status: ${JSON.stringify(workdirTracking.workdirStatus)}`)
              const [isUpdateNeeded,methodUuid,dataUuid] = workdirTracking.versions.isWorkdirStatusUpdateNeeded(workdirTracking.workdirStatus);
              //console.log(`isUpdateNeeded: ${isUpdateNeeded}, methodUuid: ${methodUuid}, dataUuid: ${dataUuid}`);
              if (isUpdateNeeded) {
                VSCode.postMessage( new RequestWorkdirStatus(message.workdirIdx, methodUuid, dataUuid) );
              }
            }
            // Update activeWorkdirIdx (as needed).
            if (common.current.activeWorkdir !== message.json.asuiSelection) {
                const idx = WORKDIRS_KEYS.indexOf(message.json.asuiSelection);
                if (idx >= 0) {
                    common.current.activeWorkdir = message.json.asuiSelection;
                    common.current.activeWorkdirIdx = idx;
                    //console.log(`Active workdir changed to ${common.current.activeWorkdir}`);
                    do_render = true;
                } else {
                    console.error(`Invalid active workdir: ${message.json.asuiSelection}`);
                }
            }
            
            
            break;
          }
          case 'UpdateWorkdirStatus': {
            const workdirTracking = workdirs.current[message.workdirIdx];
            const hasChanged = workdirTracking.workdirStatus.update(message.json);
            if (hasChanged) {
              workdirTracking.updateCalculatedFields();
              do_render = true;
            }
            break;
          }
          default:
            console.log('Received an unknown command', message);
        }
        if (do_render) {
          setUpdateTrigger(prev => !prev);
        }
      }
    } catch (error) {
      console.error("An error occurred in useCommonController:", error);
    }
  }, [message]);

  return {common, workdirs};
};