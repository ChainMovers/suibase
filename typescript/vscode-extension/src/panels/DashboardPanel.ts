import { BasePanel } from "./BasePanel";

/**
 * This class manages the state and behavior of the DashboardPanel webview.
 *
 * This is a singleton.
 */
export class DashboardPanel extends BasePanel {
  private static instance?: DashboardPanel;

  /**
   * DashboardPanel constructor called only from DashboardPanel.render()
   */
  private constructor() {
    super("suibase.settings", "Suibase Dashboard");
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
    if (!DashboardPanel.instance) {
      DashboardPanel.instance = new DashboardPanel();
    }
    DashboardPanel.instance.render();
  }

  // Dispose is a callback triggered by VSCode (see BasePanel).
  protected dispose() {
    console.log("DashboardPanel.dispose() called");
    if (DashboardPanel.instance) {
      super.dispose();
      delete DashboardPanel.instance;
      DashboardPanel.instance = undefined;
    } else {
      console.log("Error: dispose() called out of order");
    }
  }
}
