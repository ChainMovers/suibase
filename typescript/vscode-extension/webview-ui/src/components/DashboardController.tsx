// import React from "react";
import "./DashboardController.css";

//import { SuibaseJSONStorage } from "../common/SuibaseJSONStorage";
import { VSCode } from "../lib/VSCode";
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react'
import { WORKDIR_IDX_DEVNET, WORKDIR_IDX_LOCALNET, WORKDIR_IDX_MAINNET, WORKDIR_IDX_TESTNET } from "../common/Consts";
import { WorkdirCommand } from "../common/ViewMessages";
import { SuibaseJSONStorage } from "../common/SuibaseJSONStorage";
import { useMessage } from "../lib/CustomHooks";
import { useEffect } from "react";

  interface WorkdirData {
    label: string;
    workdir: string;
    workdirIdx: number,
    status: string;
    suiClientVersion: string;
    versions: SuibaseJSONStorage;
  }

  const workdirs: WorkdirData[] = [
    { label: "Localnet", workdir: "localnet", workdirIdx: WORKDIR_IDX_LOCALNET, status: "", suiClientVersion: "", versions: new SuibaseJSONStorage() },
    { label: "Devnet", workdir: "devnet", workdirIdx: WORKDIR_IDX_DEVNET, status: "", suiClientVersion: "", versions: new SuibaseJSONStorage() },
    { label: "Testnet", workdir: "testnet", workdirIdx: WORKDIR_IDX_TESTNET, status: "", suiClientVersion: "", versions: new SuibaseJSONStorage() },
    { label: "Mainnet", workdir: "mainnet", workdirIdx: WORKDIR_IDX_MAINNET, status: "", suiClientVersion: "", versions: new SuibaseJSONStorage() },
  ];

const handleStartClick = (workdir: WorkdirData) => {    
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "start"));
}

function handleStopClick(workdir: WorkdirData) {
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "stop"));
}

function handleRegenClick(workdir: WorkdirData) {
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "regen"));
} 

export const DashboardController = () => {
  const { message } = useMessage();
  
  useEffect(() => {
    // Ensure message is not null and has the expected structure
    if (message && message.name) {
      switch (message.name) {
        case 'UpdateVersions':
          // Perform actions specific to the 'refactor' command
          console.log(`UpdateVersions received ${JSON.stringify(message)}`);
          break;
        // Add more cases as needed for different commands
        default:
          console.log('Received an unknown command', message);
      }
    }
  }, [message]); // This effect runs whenever the message changes

  return (
    <>Dashboard Controller
    {workdirs.map((workdir) => (
    <div className="workdir_row" key={workdir.label}>
      <h2 className="workdir">{workdir.label}</h2>
      <h2 className="status">{workdir.status}</h2>
            {workdir.status === "Stopped" ? (
              <VSCodeButton onClick={() => handleStartClick(workdir)}>
                Start
                <span slot="start" className="codicon codicon-debug-start" />
              </VSCodeButton>
            ) : (
              <VSCodeButton onClick={() => handleStopClick(workdir)}>
                Stop
                <span slot="start" className="codicon codicon-debug-stop" />
              </VSCodeButton>
            )}

      {workdir.label === "Localnet" && (        
        <VSCodeButton onClick={() => handleRegenClick(workdir)}>
          Regen
          <span slot="start" className="codicon codicon-refresh" />
        </VSCodeButton>
      )}
    </div>
    ))}
    </>
  );
}