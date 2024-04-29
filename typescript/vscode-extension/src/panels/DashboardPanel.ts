import { BaseWebview } from "../bases/BaseWebview";
import { WEBVIEW_DASHBOARD } from "../common/Consts";

/**
 * This class manages the state and behavior of the DashboardPanel webview.
 *
 * This is a singleton.
 */
export class DashboardPanel extends BaseWebview {
  private static instance?: DashboardPanel;

  /**
   * DashboardPanel constructor called only from DashboardPanel.render()
   */
  private constructor() {
    super(WEBVIEW_DASHBOARD, "Suibase Dashboard");
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
    DashboardPanel.instance.renderPanel();
  }

  // Dispose is a callback triggered by VSCode (see BaseWebview).
  // Happens when the VSCode tab is closed.
  protected dispose() {
    //console.log("DashboardPanel.dispose() called");
    if (DashboardPanel.instance) {
      super.dispose();
      delete DashboardPanel.instance;
      DashboardPanel.instance = undefined;
    } else {
      console.log("Error: dispose() called out of order");
    }
  }

  protected handleMessage(message: any): void {
    //console.log(message);
  }
}
