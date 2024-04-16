<script lang="ts" context="module">
  import { provideVSCodeDesignSystem, allComponents } from "@vscode/webview-ui-toolkit";

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
  console.log("App Init Module Level done");
</script>

<script lang="ts">
  import { VSCode } from "./lib/VSCode";
  import WorkdirsController from "./components/DashboardController.svelte";
  import ConsoleController from "./components/ConsoleController.svelte";
  import ExplorerController from "./components/ExplorerController.svelte";
  import { ui_selected_context } from "$lib/states/L4/contexts";

  import { onMount } from "svelte";
  onMount(async () => {
    // Add a callback to connect SuibaseData with some Svelte store.
    //suibaseData.globalStates.uiSelectedContextCallback =
    console.log("App mounted component level");
  });

  function handleHowdyClick() {
    VSCode.postMessage({
      command: "hello",
      text: "Howdy!",
    });
  }

  console.log("App Init Component Level done");

  $ui_selected_context = "DSUI";
</script>

<main>
  {#if globalThis.suibase_view_key == "suibase.settings"}
    <WorkdirsController />
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <vscode-button on:click={handleHowdyClick}>Config Howdy!</vscode-button>
  {:else if globalThis.suibase_view_key == "suibase.console"}
    <ConsoleController />
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <vscode-button on:click={handleHowdyClick}>Console Howdy!</vscode-button>
  {:else if globalThis.suibase_view_key == "suibase.sidebar"}
    <ExplorerController />
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <vscode-button on:click={handleHowdyClick}>Explorer Howdy!</vscode-button>
  {/if}
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
