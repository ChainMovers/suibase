import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'

import { provideVSCodeDesignSystem, allComponents } from "@vscode/webview-ui-toolkit";
import { MessageProvider } from './lib/MessageContext.tsx';
//import { StyledEngineProvider } from '@mui/material/styles';

// In order to use the Webview UI Toolkit web components they
// must be registered with the browser (i.e. webview) using the
// syntax below.
//provideVSCodeDesignSystem().register(vsCodeButton());
provideVSCodeDesignSystem().register(allComponents);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {/*<StyledEngineProvider injectFirst>*/}
      <MessageProvider>
        <App />
      </MessageProvider>
  </React.StrictMode>,
)


