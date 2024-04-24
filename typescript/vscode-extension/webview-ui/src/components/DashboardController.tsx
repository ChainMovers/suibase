// import React from "react";
import "./DashboardController.css";

import { VSCode } from "../lib/VSCode";
import { VSCodeButton } from "@vscode/webview-ui-toolkit/react";
import { WorkdirCommand } from "../common/ViewMessages";

import { useCommonController, ViewWorkdirData } from "./CommonController";
import { Badge, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";

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
  const { common, workdirs } = useCommonController();

  return (
    <>
      <Typography variant="body1">Services</Typography>
      {/*Active = {common.current.activeWorkdir}*/}
      <TableContainer>
        <Table aria-label="Suibase Services" sx={{ minWidth: 600 }} size="small">
          <TableHead>
            <TableRow>
              <TableCell style={{ width: '100px' }}></TableCell>
              <TableCell style={{ width: '100px' }}>Status</TableCell>              
              <TableCell style={{ width: '200px' }}>{/* Controls */}</TableCell>
              <TableCell style={{ width: '100px' }}>Version</TableCell>
              <TableCell style={{ width: '100px' }}>More Controls</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
          {workdirs.current.map((workdir,index) => (
            <TableRow key={workdir.label} sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
              <TableCell>
                <Badge variant="dot" color="info" anchorOrigin={{vertical: 'top', horizontal: 'left',}} invisible={!workdir.isStatusLoaded || index !== common.current.activeWorkdirIdx}>
                  <Typography variant="body2">{workdir.label}</Typography>
                </Badge>
              </TableCell>
              <TableCell>
                <Typography variant="h6">{workdir.status}</Typography>
              </TableCell>              
              <TableCell> 
                {workdir.isStatusLoaded && (
                  <>                
                 {workdir.status === "STOPPED" ? (
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
                </>
              )}              
              </TableCell>   
              <TableCell>
                {workdir.isStatusLoaded && workdir.suiClientVersionShort}
              </TableCell>         
              <TableCell>
                {workdir.label === "Localnet" && (
                  <VSCodeButton onClick={() => handleRegenClick(workdir)}>
                    Regen
                    <span slot="start" className="codicon codicon-refresh" />
                  </VSCodeButton>              
                )}
              </TableCell>
            </TableRow>
          ))}
          </TableBody>
        </Table>
      </TableContainer>
    </>
  );
};
