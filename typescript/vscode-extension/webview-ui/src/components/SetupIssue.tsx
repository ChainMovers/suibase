// Convert a setupIssue string (these are coming through extension messages) into 
// a user friendly message with links.
import { Link, Typography } from "@mui/material";
import { SETUP_ISSUE_GIT_NOT_INSTALLED, SETUP_ISSUE_SUIBASE_NOT_INSTALLED } from "../../../src/common/Consts";
import { SETUP_ISSUE_SUIBASE_NOT_ON_PATH } from "../common/Consts";

// with a link to "Check https://suibase.io/how-to/install" on next line.
export default function SetupIssue(props: {issue: string}) {    
  let issue = props.issue;

  // Extract from the issue string an appended ",homedir=<path>" if any.
  let homedir = "";
  const match = issue.match(/,homedir=(.*)/);
  if (match) {
    homedir = match[1];
    issue = issue.replace(/,homedir=(.*)/, "");
  }
  
  if (issue == SETUP_ISSUE_SUIBASE_NOT_INSTALLED) {
    return (
      <Typography variant="body2">
        {SETUP_ISSUE_SUIBASE_NOT_INSTALLED}<br/>
        Check <Link href="https://suibase.io/how-to/install">https://suibase.io/how-to/install</Link>
      </Typography>
    );
  } else if (issue == SETUP_ISSUE_GIT_NOT_INSTALLED) {
    return (
      <Typography variant="body2">
        {SETUP_ISSUE_GIT_NOT_INSTALLED}<br/>
        Please install Sui prerequisites<br/>
        Check <Link href="https://docs.sui.io/guides/developer/getting-started/sui-install">https://docs.sui.io</Link>
      </Typography>
    );
  } else if (issue == SETUP_ISSUE_SUIBASE_NOT_ON_PATH) {
    return (
      <Typography variant="body2">
        {SETUP_ISSUE_SUIBASE_NOT_ON_PATH}<br/>
        Initialize your shell to have {homedir}/.local/bin added to the $PATH.<br/>
        Check <Link href="https://suibase.io/how-to/install">https://suibase.io/how-to/install</Link>
      </Typography>
    );
  }

  return (
    <Typography variant="body2">{issue}</Typography>
  );
}
