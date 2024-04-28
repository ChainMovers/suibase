// import React from "react";
import "./styles/DashboardController.css";

import { VSCode } from "../lib/VSCode";
import { VSCodeButton } from "@vscode/webview-ui-toolkit/react";
import { WorkdirCommand } from "../common/ViewMessages";

import { useCommonController, ViewWorkdirStates } from "./CommonController";
import { Badge, Box, CircularProgress, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";
import { AntSwitch } from "./AntSwitch";
import { useEffect, useState } from "react";
import { WORKDIRS_KEYS, WORKDIRS_LABELS } from "../common/Consts";

function handleRegenClick(workdir: ViewWorkdirStates) {
  VSCode.postMessage(new WorkdirCommand(workdir.workdirIdx, "regen"));
}

// One instance per element in WORKDIRS_KEYS.
class WorkdirStates {
  public showSpinner: boolean; // States that considers all other spinner states.

  // Note: the backend states are in workdirs.current

  // Switch related states.
  public spinnerForSwitch: boolean;
  
  public requestedChange: boolean | undefined; // undefined if no request
  public switchState: boolean; // The state shown in the UI (may not match backend).
  public switchSkeleton: boolean; // The switch should not display any state.

  // Version update related states.
  public spinnerForUpdate: boolean; 

  constructor() {
    this.showSpinner = false;
    this.spinnerForSwitch = false;
    this.spinnerForUpdate = false;
    this.requestedChange = undefined;
    this.switchState = false;
    this.switchSkeleton = true;
  }
}

export const DashboardController = () => {
  const { commonTrigger, common, workdirs } = useCommonController();

  const switchProps = { inputProps: { 'aria-label': 'workdir on/off' } };

  // Calculated states that consider both the backend and the user requests.
  const [workdirStates, setWorkdirStates] = useState<WorkdirStates[]>(WORKDIRS_KEYS.map(() => new WorkdirStates()));
  const updateWorkdirStates = (index: number, updates: Partial<WorkdirStates>) => {
    setWorkdirStates(currentStates =>
      currentStates.map((item, idx) =>
        idx === index ? { ...item, ...updates } : item
      )
    );
  };

  const toSwitchState = ( workdirStates: ViewWorkdirStates ): boolean => {
    switch (workdirStates.workdirStatus.status) {
      case "DEGRADED":
      case "OK":
        return true;
      /*
      case "DOWN":
      case "STOPPED":        
        break;*/
      default:
        return false;      
    }
}
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleSwitchChange = (index: number, newValue: boolean) => {
    // Get the related workdir backend states.
    const workdirStates = workdirs[index];
    const switchBackendState = toSwitchState(workdirStates);
    if (newValue !== switchBackendState) {
      updateWorkdirStates(index, { requestedChange: newValue, switchState: newValue, spinnerForSwitch: true });
      // Do CLI "start" or "stop" for the requested workdir.
      const command = newValue? "start" : "stop";
      VSCode.postMessage(new WorkdirCommand(index, command));      
    } else {
      // Matching with backend. Clear the request, make sure the UI matches.      
      updateWorkdirStates(index, { requestedChange: undefined, switchState: switchBackendState, spinnerForSwitch: false });
    }
  };

  // Reconciliate backend and UI whenever something changes...
  useEffect(() => {
    // Iterate every workdirStates.
    workdirStates.forEach((state, index) => {
      const workdirStates = workdirs[index];
      const switchBackendState = toSwitchState(workdirStates);
      if (state.requestedChange !== undefined) {
        // User requested a change, keep it that way until the backend confirms.
        if (state.requestedChange === switchBackendState) {
          // Matching with backend. Clear the request, make sure the UI matches.
          updateWorkdirStates(index, { requestedChange: undefined, switchState: switchBackendState, spinnerForSwitch: false });
        } else {
          // Pending request... get the spinner spinning.
          if (!state.spinnerForSwitch) {
            updateWorkdirStates(index, { spinnerForSwitch: true });
          }
        }
      } else {
        // No request, just make sure the UI match the backend.
        if (state.switchState !== switchBackendState) {
          updateWorkdirStates(index, { switchState: switchBackendState, spinnerForSwitch: false });
        }
      }

      // Calculated spinner state
      const expectedSpinnerState = (state.spinnerForSwitch || state.spinnerForUpdate);
      if (state.showSpinner !== expectedSpinnerState) {
        updateWorkdirStates(index, {showSpinner: expectedSpinnerState});
      }
    });

    return () => {};
  }, [commonTrigger,workdirs,workdirStates]);

  return (
    <>
      <Typography variant="body1">Services</Typography>
      {/*Active = {common.current.activeWorkdir}*/}
      <TableContainer>
        <Table aria-label="Suibase Services" sx={{ minWidth: 420, maxWidth: 420 }} size="small">
          <TableHead>
            <TableRow>
              <TableCell style={{ width: '115px' }}></TableCell>
              <TableCell align="center" style={{ width: '105px' }}>Status</TableCell>
              <TableCell style={{ width: '100px' }}>Version</TableCell>
              <TableCell style={{ width: '100px' }}>More Controls</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
          {workdirStates.map((workdirState,index) => {
            const workdirStates = workdirs[index];
            return (
            <TableRow key={WORKDIRS_LABELS[index]} sx={{ p: 0, m: 0, '&:last-child td, &:last-child th': { border: 0 } }}>
              <TableCell sx={{ width: 115, maxWidth: 115, p: 0, m: 0}}>
                <Box display="flex" alignItems="center" flexWrap="nowrap">
                  <Box width="10px" display="flex" justifyContent="left" alignItems="center">
                    {workdirState.showSpinner && <CircularProgress size={9}/>}
                  </Box>                          
                  <Box width="50px" display="flex" justifyContent="center" alignItems="center">
                    <AntSwitch {...switchProps} checked={workdirState.switchState} onChange={(event) => handleSwitchChange(index, event.target.checked)}/>
                  </Box>                       
                  <Box width="50px" display="flex" justifyContent="left" alignItems="center">
                    <Badge variant="dot" color="info" anchorOrigin={{vertical: 'top', horizontal: 'left',}} invisible={!workdirStates.workdirStatus.isLoaded || index !== common.current.activeWorkdirIdx}>
                      <Typography variant="body2" sx={{pl:'2px'}}>{WORKDIRS_LABELS[index]}</Typography>
                    </Badge>
                  </Box>
                </Box>
              </TableCell>
              <TableCell align="center" sx={{ width: 105, maxWidth: 105, p: 0, m: 0}}>
                <Typography variant="subtitle2">{workdirStates.workdirStatus.status}</Typography>                
              </TableCell>              
              <TableCell>
                <Typography variant="body2">{workdirStates.workdirStatus.isLoaded && workdirStates.workdirStatus.suiClientVersionShort}</Typography>
              </TableCell>         
              <TableCell>
                {workdirStates.label === "Localnet" && (
                  <VSCodeButton onClick={() => handleRegenClick(workdirStates)}>
                    Regen
                    <span slot="start" className="codicon codicon-refresh" />
                  </VSCodeButton>              
                )}
              </TableCell>
            </TableRow>
            );
          })}
          </TableBody>
        </Table>
      </TableContainer>
      {/*<DebugTreeViewObj jsonObj={workdirs.current}/>*/}
    </>
  );
};
