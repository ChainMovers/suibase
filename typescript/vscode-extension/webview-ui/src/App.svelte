<script lang="ts" context="module">
  import { provideVSCodeDesignSystem, allComponents } from "@vscode/webview-ui-toolkit";
  import { VSCode } from "./lib/VSCode";
  import WorkdirsController from "./components/DashboardController.svelte";
  import ConsoleController from "./components/ConsoleController.svelte";
  import ExplorerController from "./components/ExplorerController.svelte";

  // In order to use the Webview UI Toolkit web components they
  // must be registered with the browser (i.e. webview) using the
  // syntax below.
  //provideVSCodeDesignSystem().register(vsCodeButton());
  provideVSCodeDesignSystem().register(allComponents);

  // Instantiate the StateLoop singleton (will also start its loop).

  // To register more toolkit components, simply import the component
  // registration function and call it from within the register
  // function, like so:
  //
  // provideVSCodeDesignSystem().register(
  //   vsCodeButton(),
  //   vsCodeCheckbox()
  // );
  //
  // Finally, if you would like to register all of the toolkit
  // components at once, there's a handy convenience function:
  //
  // provideVSCodeDesignSystem().register(allComponents);

  function handleHowdyClick() {
    VSCode.postMessage({
      command: "hello",
      text: "Hey there partner! 🤠",
    });
  }
</script>

<main>
  {#if globalThis.suibase_panel_key == "suibase.settings"}
    <WorkdirsController />
  {:else if globalThis.suibase_panel_key == "suibase.console"}
    <ConsoleController />
  {:else if globalThis.suibase_panel_key == "explorer.console"}
    <ExplorerController />
  {/if}

  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <vscode-button on:click={handleHowdyClick}>Howdy!</vscode-button>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    height: 100%;
  }
</style>
