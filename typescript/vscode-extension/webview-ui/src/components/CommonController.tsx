// This is a custom Hook used by all other view/controller components.
//
// It keep tracks of the "versions" for each workdir and 
// "status" data for the active workdir.
import { useRef, useState, useEffect } from 'react';
import { useMessage } from "../lib/CustomHooks";
import { SuibaseJson } from "../common/SuibaseJson";
import { WORKDIRS_KEYS } from "../../../src/common/Consts";
import { VSCode } from '../lib/VSCode';
import { InitView } from '../../../src/common/ViewMessages';

export class ViewWorkdirData {
    label: string;
    workdir: string;
    workdirIdx: number;
    status: string;
    suiClientVersion: string;
    versions: SuibaseJson;

    constructor(workdir: string, workdirIdx: number) {
      this.label = workdir.charAt(0).toUpperCase() + workdir.slice(1);      
      this.workdir = workdir;
      this.workdirIdx = workdirIdx;
      this.status = "";
      this.suiClientVersion = "";
      this.versions = new SuibaseJson();
    }
  }

export class ViewCommonData {
    public activeWorkdir: string;
    public activeWorkdirIdx: number;

    constructor() {
      this.activeWorkdir = "localnet";
      this.activeWorkdirIdx = 0;
    }
}

export const useCommonController = () => {
  const { message } = useMessage();
  const common = useRef(new ViewCommonData());
  const workdirs = useRef(WORKDIRS_KEYS.map((key, index) => new ViewWorkdirData(key, index)));
  const [, setUpdateTrigger] = useState(false);

  useEffect(() => {    
    VSCode.postMessage(new InitView());
  }, []);

  useEffect(() => {
    try {
      if (message && message.name) {
        let do_render = false;
        switch (message.name) {
          case 'UpdateVersions': {
            const hasChanged = workdirs.current[message.workdirIdx].versions.update(
              message.json.header.methodUuid,
              message.json.header.dataUuid,
              message.json
            );
            if (hasChanged) {
              do_render = true;
            }
            // Update activeWorkdirIdx (as needed).
            if (common.current.activeWorkdir !== message.json.asuiSelection) {
                const idx = WORKDIRS_KEYS.indexOf(message.json.asuiSelection);
                if (idx >= 0) {
                    common.current.activeWorkdir = message.json.asuiSelection;
                    common.current.activeWorkdirIdx = idx;
                    console.log(`Active workdir changed to ${common.current.activeWorkdir}`);
                    do_render = true;
                } else {
                    console.error(`Invalid active workdir: ${message.json.asuiSelection}`);
                }
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
      console.error("An error occurred in useWorkdirTracking:", error);
    }
  }, [message]);

  return {common, workdirs};
};