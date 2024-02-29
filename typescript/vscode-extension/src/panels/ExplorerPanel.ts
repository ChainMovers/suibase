import { BasePanel } from "./BasePanel";

/**
 * This class manages the state and behavior of the DashboardPanel webview.
 *
 * This is a singleton.
 */
export class ExplorerPanel extends BasePanel {
  private static instance?: ExplorerPanel;

  /**
   * ExplorerPanel constructor called only from ExplorerPanel.render()
   */
  private constructor() {
    super("suibase.explorer", "Suibase Explorer");
  }

  // Note: Does not use the activate/deactivate pattern (the BasePanel does).
  //       Instead this subclass uses a render()/dispose() for its lifetime.
  //
  //       This is because activate() always happens once and early while render()
  //       and dispose() may happen or not depending of the user actions to display
  //       the panel or not.
  //
  public static render() {
    //DashboardPanel.instance = DashboardPanel.instance ?? new DashboardPanel();
    if (!ExplorerPanel.instance) {
      ExplorerPanel.instance = new ExplorerPanel();
    }
    ExplorerPanel.instance.render();
  }

  // Dispose is a callback triggered by VSCode (see BasePanel).
  protected dispose() {
    console.log("ExplorerPanel.dispose() called");
    if (ExplorerPanel.instance) {
      super.dispose();
      delete ExplorerPanel.instance;
      ExplorerPanel.instance = undefined;
    } else {
      console.log("Error: dispose() called out of order");
    }
  }
}
