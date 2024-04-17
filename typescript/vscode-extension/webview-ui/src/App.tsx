import './App.css'
import { ConsoleController } from './components/ConsoleController';
import { ExplorerController } from './components/ExplorerController';
import { DashboardController } from "./components/DashboardController";

function App() {

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
