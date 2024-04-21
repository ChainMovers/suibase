
import './App.css'

import { useEffect } from 'react';
import { useMessage } from './lib/CustomHooks';

import { ConsoleController } from './components/ConsoleController';
import { ExplorerController } from './components/ExplorerController';
import { DashboardController } from "./components/DashboardController";


function App() {
  const { setMessage } = useMessage();

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      // Can do some initial handling here before
      // setting the react state for the children.
      setMessage(event.data);
    }

    window.addEventListener('message', handleMessage); 
    return () => window.removeEventListener('message', handleMessage);
  }, [setMessage]);

  let controller;
  switch (globalThis.suibase_view_key) {
    case "suibase.settings":
      controller = <DashboardController />;
      break;
    case "suibase.console":
      controller = <ConsoleController />;
      break;
    case "suibase.sidebar":
      controller = <ExplorerController />;
      break;
    default:
      controller = null;
  }
  
  return (
    <main> {controller}</main>
  );
}

export default App
