import { BaseWebview } from "../bases/BaseWebview";

/**
 * This class manages the state and behavior of the ConsolePanel webview.
 *
 * This is a singleton.
 */
export class ConsolePanel extends BaseWebview {
  private static instance?: ConsolePanel;

  /**
   * ConsolePanel constructor called only from ConsolePanel.render()
   */
  private constructor() {
    super("suibase.console", "Sui Console");
  }

  // Note: Does not use the activate/deactivate pattern (the BaseWebview does).
  //       Instead this subclass uses a render()/dispose() for its lifetime.
  //
  //       This is because activate() always happens once and early while render()
  //       and dispose() may happen or not depending of the user actions to display
  //       the panel or not.
  //
  public static render() {
    if (!ConsolePanel.instance) {
      ConsolePanel.instance = new ConsolePanel();
    }
    ConsolePanel.instance.renderPanel();
  }

  // Dispose is a callback triggered by VSCode (see BaseView).
  protected dispose() {
    console.log("ConsolePanel.dispose() called");
    if (ConsolePanel.instance) {
      super.dispose();
      delete ConsolePanel.instance;
      ConsolePanel.instance = undefined;
    } else {
      console.log("Error: dispose() called out of order");
    }
  }
}
