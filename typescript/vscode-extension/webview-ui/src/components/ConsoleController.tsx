import { Box } from "@mui/material";
import { purple } from "@mui/material/colors";

const cssVar = (variableName: string) => {
  try {
  // Get the computed style of the :root element (document.documentElement)
  const value = getComputedStyle(document.documentElement).getPropertyValue(variableName).trim();  
  return value || purple[500];
  } catch (error) {
    console.error(`Error getting CSS variable ${variableName}: ${error}`);
    return purple[500];
  }
};

const vsColor = (color: any) => {  
  
  let colorVar = color.replace(/\./g, '-').toLowerCase();
  // Append --vscode- to colorVar if not already present.
  if (!colorVar.startsWith("--vscode-")) {
    colorVar = "--vscode-" + colorVar;
  }  
  const value = cssVar(`${colorVar}`);
  return (
    <Box display="flex" alignItems="center" gap={1}>
      <div>{colorVar}: {JSON.stringify(value)}</div>
      <Box sx={{
      width: '10px',
      height: '10px',
      backgroundColor: value,
      border: '1px solid #000',
    }} />
    </Box>
  );    
}

export const ConsoleController = () => {

  // Data exchanged with the extension.
  //let suibaseData: SuibaseData = SuibaseData.getInstance();

  return (
    <>Note: Not implemented yet.<br/> Will show events from your last published module(s).    

        {vsColor("editor-foreground")}
        {vsColor("editor-background")}        
        {vsColor("badge-background")}
        {vsColor("badge-foreground")}
    </>
  );
}

/*
<script lang="ts">
  import { VSCode } from "../lib/VSCode";
  import { SuibaseJSONStorage } from "../common/SuibaseJSONStorage";
</script>

<main>
  <ul class="no-bullets">
    <li>
      2023-10-25 14:46:19.031 [info] > git show 
      :typescript/vscode-extension/webview-ui/src/components/ConsoleController.svelte [21ms]
    </li>


    <li>2023-10-25 14:53:26.114 [info] > git fetch [536ms]</li>

    <li>2023-10-25 14:53:26.145 [info] > git config --get commit.template [1ms]</li>
</ul>
</main>
*/