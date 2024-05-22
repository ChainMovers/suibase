// import React from "react";
import "./styles/DashboardController.css";

import { VSCode } from "../lib/VSCode";
//import { VSCodeButton } from "@vscode/webview-ui-toolkit/react";
import { WorkdirCommand } from "../common/ViewMessages";

import { useCommonController, ViewWorkdirData } from "./CommonController";
import { Badge, Box, CircularProgress, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from "@mui/material";
import { AntSwitch } from "./AntSwitch";
import { useEffect, useState } from "react";
import { WORKDIRS_KEYS, WORKDIRS_LABELS } from "../common/Consts";
import { WEBVIEW_DASHBOARD } from "../../../src/common/Consts";
import SetupIssue from "./SetupIssue";
/*
function handleRegenClick(workdir: ViewWorkdirStates) {
  VSCode.postMessage(new WorkdirCommand(WEBVIEW_DASHBOARD,workdir.workdirIdx, "regen"));
}*/

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
  const { commonTrigger, statusTrigger, common, workdirs } = useCommonController(WEBVIEW_DASHBOARD, { trackStatus: true });

  const switchProps = { inputProps: { 'aria-label': 'workdir on/off' } };

  const [allDisabled, setAllDisabled] = useState(false);
  const [workdirStates, setWorkdirStates] = useState<WorkdirStates[]>(WORKDIRS_KEYS.map(() => new WorkdirStates()));

  const updateWorkdirStates = (index: number, updates: Partial<WorkdirStates>) => {
    setWorkdirStates(currentStates =>
      currentStates.map((item, idx) =>
        idx === index ? { ...item, ...updates } : item
      )
    );
  };

  const toSwitchState = ( workdirData: ViewWorkdirData ): boolean => {
    switch (workdirData.workdirStatus.status) {
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
  
  const handleSwitchChange = (index: number, newValue: boolean) => {
    // Get the related workdir backend states.
    const workdirData = workdirs[index];
    const switchBackendState = toSwitchState(workdirData);
    if (newValue !== switchBackendState) {
      updateWorkdirStates(index, { requestedChange: newValue, switchState: newValue, spinnerForSwitch: true });
      // Do CLI "start" or "stop" for the requested workdir.
      const command = newValue? "start" : "stop";
      VSCode.postMessage(new WorkdirCommand(WEBVIEW_DASHBOARD,index, command));      
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
  }, [commonTrigger,statusTrigger,workdirs,workdirStates]);

  useEffect(() => {
    // Check if all workdirs are disabled.
    let allDisabledCalc = true;
    for (let i = 0; i < WORKDIRS_KEYS.length; i++) {
      if (workdirs[i].workdirStatus.status !== "DISABLED") {
        allDisabledCalc = false;
        break;
      }
    }
    // Update the state.
    if (allDisabledCalc !== allDisabled) {
      setAllDisabled(allDisabledCalc);
    }
  }, [workdirs, statusTrigger]);

  return (
      <Box sx={{paddingLeft:1}}>
      {common.current.setupIssue && <SetupIssue issue={common.current.setupIssue}/>}
      {common.current.activeLoaded && !common.current.setupIssue? (
        <>
      <Typography variant="body1">Services</Typography>
      <TableContainer>
        <Table aria-label="Suibase Services" sx={{ minWidth: 420, maxWidth: 420 }} size="small">
          <TableHead>
            <TableRow>
              <TableCell style={{ width: '115px' }}></TableCell>
              <TableCell align="center" style={{ width: '105px' }}>Status</TableCell>
              <TableCell align="center" style={{ width: '100px' }}>Version</TableCell>
              <TableCell style={{ width: '100px' }}>{/*More Controls*/}</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
          {workdirStates.map((workdirState,index) => {
            const viewData = workdirs[index];
            const badgeInvisible = allDisabled || !viewData.workdirStatus.isLoaded || index !== common.current.activeWorkdirIdx;
            return (
            <TableRow key={WORKDIRS_LABELS[index]} sx={{p: 0, m: 0, '&:last-child td, &:last-child th': { border: 0 } }}>
              <TableCell sx={{width: 115, maxWidth: 115, pt: '6px', pb: '6px', pl: 0, pr: 0, m: 0}}>
                <Box display="flex" alignItems="center" flexWrap="nowrap">
                  <Box width="10px" display="flex" justifyContent="left" alignItems="center">
                    {workdirState.showSpinner && <CircularProgress size={9}/>}
                  </Box>                          
                  <Box width="50px" display="flex" justifyContent="center" alignItems="center">
                    <AntSwitch {...switchProps} size="small" disabled={workdirState.showSpinner} checked={workdirState.switchState} onChange={(event) => handleSwitchChange(index, event.target.checked)}/>
                  </Box>                       
                  <Box width="50px" display="flex" justifyContent="left" alignItems="center">
                    <Badge variant="dot" color="info" anchorOrigin={{vertical: 'top', horizontal: 'left',}} invisible={badgeInvisible}>
                      <Typography variant="body2" sx={{pl:'2px'}}>{WORKDIRS_LABELS[index]}</Typography>
                    </Badge>
                  </Box>
                </Box>
              </TableCell>
              <TableCell align="center" sx={{width: 105, maxWidth: 105, p: 0, m: 0}}>
                <Typography variant="subtitle2">{viewData.workdirStatus.status}</Typography>                
              </TableCell>              
              <TableCell align="center" sx={{width: 65, maxWidth: 65, p: 0, m: 0}}>
                <Typography variant="body2">{viewData.workdirStatus.isLoaded && viewData.workdirStatus.suiClientVersionShort}</Typography>
              </TableCell>         
              <TableCell>
                {/* Not supported for now
                {workdirStates.label === "Localnet" && (
                  <VSCodeButton onClick={() => handleRegenClick(workdirStates)}>
                    Regen
                    <span slot="start" className="codicon codicon-refresh" />
                  </VSCodeButton>              
                )}*/}
              </TableCell>
            </TableRow>
            );
          })}
          </TableBody>
        </Table>
      </TableContainer>
      </>
      ) : (<CircularProgress size={15}/>)
      }
    </Box>
  );
};
