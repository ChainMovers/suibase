
import './App.css'

import { useEffect, useState } from 'react';
import { useMessage } from './lib/CustomHooks';

import { ConsoleController } from './components/ConsoleController';
import { ExplorerController } from './components/ExplorerController';
import { DashboardController } from "./components/DashboardController";

import { createTheme} from '@mui/material/styles';
import { CssBaseline, ThemeProvider } from '@mui/material';
import React from 'react';
import { purple } from '@mui/material/colors';
import { WEBVIEW_CONSOLE, WEBVIEW_DASHBOARD, WEBVIEW_EXPLORER } from '../../src/common/Consts';

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

const useExternalCssVariable = (variableName: string) => {
  const [value, setValue] = useState(getComputedStyle(document.documentElement).getPropertyValue(variableName));

  
  useEffect(() => {
    const intervalId = setInterval(() => {
      const newValue = getComputedStyle(document.documentElement).getPropertyValue(variableName);
      if (newValue !== value) {
        setValue(newValue);
      }
    }, 1000); // Check every 1 seconds

    return () => clearInterval(intervalId);
  }, [value, variableName]);

  return value;
};

function App() {
  const { setMessage } = useMessage();

  // Observe a single vscode CSS var periodically to trigger a refresh of the MUI palette.
  const vscodeThemeChange = useExternalCssVariable('--vscode-editor-foreground');

  // See https://code.visualstudio.com/api/references/theme-color
  const adaptiveTheme = React.useMemo(() => createTheme({
    typography: {
      fontSize: 12,
    }, 
    palette: {
      /*  primary?: PaletteColorOptions;
  secondary?: PaletteColorOptions;
  error?: PaletteColorOptions;
  warning?: PaletteColorOptions;
  info?: PaletteColorOptions;
  success?: PaletteColorOptions;
  mode?: PaletteMode;
  tonalOffset?: PaletteTonalOffset;
  contrastThreshold?: number;
  common?: Partial<CommonColors>;
  grey?: ColorPartial;
  text?: Partial<TypeText>;
  divider?: string;
  action?: Partial<TypeAction>;
  background?: Partial<TypeBackground>;
  */
      background: {
        default: cssVar("--vscode-editor-background"),
      },
      text: {
        primary: cssVar("--vscode-editor-foreground"),
      },
      primary: {
        //main: 'var(--vscode-editor-foreground)',
        main: cssVar("--vscode-editor-foreground"),
      },
      secondary: {
        //main: 'var(--vscode-editor-foreground)',
        main: cssVar("--vscode-editor-foreground"),
      },
    },
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [vscodeThemeChange]);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      if (event.data) {
        setMessage(event.data);
      }             
    }    
    window.addEventListener('message', handleMessage); 
    return () => window.removeEventListener('message', handleMessage);    
  }, [setMessage]);

  let controller;
  switch (globalThis.suibase_view_key) {
    case WEBVIEW_DASHBOARD:
      controller = <DashboardController />;
      break;
    case WEBVIEW_CONSOLE:
      controller = <ConsoleController />;
      break;
    case WEBVIEW_EXPLORER:
      controller = <ExplorerController />;
      break;
    default:
      controller = null;
  }
  
  return (
    <ThemeProvider theme={adaptiveTheme}>
      <CssBaseline/>
      <main>{controller}</main>
    </ThemeProvider>
  );
}

export default App
