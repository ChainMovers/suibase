import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'
import { MessageProvider } from './lib/MessageContext.tsx';

import { provideVSCodeDesignSystem, allComponents } from "@vscode/webview-ui-toolkit";
//import { purple } from '@mui/material/colors';
provideVSCodeDesignSystem().register(allComponents);

//import '@fontsource/roboto/300.css';
//import '@fontsource/roboto/400.css';
//import '@fontsource/roboto/500.css';
//import '@fontsource/roboto/700.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" />
<link
  rel="stylesheet"
  href="https://fonts.googleapis.com/css2?family=Roboto:wght@300;400;500;700&display=swap"
/>  
  <React.StrictMode>
      <MessageProvider>
        <App />
      </MessageProvider>
  </React.StrictMode>
  </>
)


