// import React from "react";
import "./DashboardController.css";

import { VSCode } from "../lib/VSCode";
import { VSCodeButton } from '@vscode/webview-ui-toolkit/react'
import { WorkdirCommand } from "../common/ViewMessages";

import { useCommonController, ViewWorkdirData } from "./CommonController";

const handleStartClick = (workdir: ViewWorkdirData) => {    
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "start"));
}

function handleStopClick(workdir: ViewWorkdirData) {
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "stop"));
}

function handleRegenClick(workdir: ViewWorkdirData) {
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "regen"));
} 

export const DashboardController = () => {
  const workdirs = useCommonController();  

  return (
    <>Dashboard Controller
    
    {workdirs.current.map((workdir) => (
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