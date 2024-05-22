
import { useCommonController } from "./CommonController";
import { WORKDIRS_LABELS, WORKDIRS_KEYS } from "../common/Consts";
import { useEffect, useState } from "react";
import { VSCodeDropdown, VSCodeOption } from "@vscode/webview-ui-toolkit/react";
import { Box, CircularProgress, Link, Typography } from "@mui/material";
import { VSCode } from "../lib/VSCode";
import { WorkdirCommand, OpenDiagnosticPanel } from "../common/ViewMessages";
import { WEBVIEW_EXPLORER } from "../../../src/common/Consts";
import { ExplorerTreeView } from "./ExplorerTreeView";
import SetupIssue from "./SetupIssue";

export const ExplorerController = () => {
  const {common, workdirs, statusTrigger, commonTrigger, packagesTrigger} = useCommonController(WEBVIEW_EXPLORER, {trackStatus: true, trackPackages: true});

  const [requestedActive, setRequestedActive] = useState("");
  const [dropdownActive, setDropdownActive] = useState(common.current.activeWorkdir);
  const [allDisabled, setAllDisabled] = useState(false);
  
  const handleDropdownChange = (event: any) => {
    const newValue = event.target.value;
    if (newValue !== common.current.activeWorkdir) {    
      setRequestedActive(newValue);
      setDropdownActive(newValue);
      // Do CLI "set-active" to the requested workdir.
      const workdirIdx = WORKDIRS_KEYS.indexOf(newValue);
      if (workdirIdx !== -1) {
        VSCode.postMessage(new WorkdirCommand(WEBVIEW_EXPLORER,workdirIdx, "set-active"));
      }
      
    } else {
      setRequestedActive("");
      setDropdownActive(common.current.activeWorkdir);
    }
  };

  useEffect(() => {
    if (requestedActive !== "") {
      // User requested a change, keep it that way until the backend confirms.
      if (requestedActive === common.current.activeWorkdir) {
        // Matching with backend. Clear the request, make sure the UI matches.
        setRequestedActive("");
        setDropdownActive(common.current.activeWorkdir);
      }
    } else {
      // No request, so match the backend.
      if (dropdownActive !== common.current.activeWorkdir) {
        setDropdownActive(common.current.activeWorkdir);
      }
    }
    return () => {};
  }, [requestedActive, dropdownActive, common.current.activeWorkdir, commonTrigger]);

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

  const renderDropdown = () => {
    return (
        <Box flexDirection="column" justifyContent="center" width="100%" paddingLeft={1} paddingTop={1}>
          {common.current.activeLoaded ? (
            <>
              <VSCodeDropdown value={dropdownActive} onChange={handleDropdownChange}>
                {WORKDIRS_KEYS.map((key,index) => {                
                  const viewData = workdirs[index];
                  const isSelected = (key === dropdownActive);
                  const isDisabled = !isSelected && (viewData.workdirStatus.status === "DISABLED");
                  return (
                    <VSCodeOption
                      key={key}
                      value={key}
                      selected={isSelected}
                      disabled={isDisabled}
                    >
                      {WORKDIRS_LABELS[index]}
                    </VSCodeOption>
                  );
                })}
              </VSCodeDropdown>
              {requestedActive && <CircularProgress size={15} style={{ marginLeft: '3px' }}/>}
            </>
          ) : (<CircularProgress size={15}/>)
          }
        </Box>

    );
  }

  const renderCommunityLink = () => {
    return (
      <Box display="flex" justifyContent="center" width="100%" paddingTop={1}>
        <Typography variant="caption" sx={{ alignContent: 'center', fontSize: '9px' }}>Need help? Try the&nbsp;
          <Link color='inherit' href="https://suibase.io/community/" target="_blank" rel="noopener noreferrer">sui community</Link>
        </Typography>
      </Box>
    );
  }

  const renderTreeView = () => {
    return (
      <Box width="100%" paddingTop={1}>
        {common.current.activeLoaded && 
          <ExplorerTreeView packagesTrigger={packagesTrigger}
                            packagesJson={workdirs[common.current.activeWorkdirIdx].workdirPackages} 
                            workdir={common.current.activeWorkdir}
                            workdirIdx={common.current.activeWorkdirIdx}/>
        }
      </Box>
    );
  }

  const renderAllDisabledHelp = () => {
    return (
      <Box display="flex" justifyContent="center" width="100%" paddingTop={1}>
        <Typography variant="body2">
          There is no workdir enabled. Do 'testnet start' command in a terminal or try the&nbsp;
          <Link 
            component="button"
            variant="body2"
            onClick={() => {
              // Post a message to the extension
              VSCode.postMessage(new OpenDiagnosticPanel());
            }}
          >
          dashboard
          </Link>.
        </Typography>
      </Box>
    );
  }

  const renderControls = !common.current.setupIssue && !allDisabled;

  return (
        <>
        {common.current.setupIssue && <SetupIssue issue={common.current.setupIssue}/>}
        
        {allDisabled && renderAllDisabledHelp()}

        {renderControls && renderDropdown()}

        {renderCommunityLink()}

        {renderControls && renderTreeView()}
        </>
  );
}