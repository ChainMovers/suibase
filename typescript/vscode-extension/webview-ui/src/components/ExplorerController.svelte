<script lang="ts">
  import { VSCode } from "../lib/VSCode";
  import { onMount } from "svelte";
  import { SuibaseData } from "../common/SuibaseData";
  import { all_contexts, ui_selected_context } from "$lib/states/L4/contexts";

  // Data exchanged with the extension.
  let suibaseData: SuibaseData = SuibaseData.getInstance();

  onMount(() => {
    // Add a callback to connect SuibaseData with some Svelte store.
    //suibaseData.globalStates.uiSelectedContextCallback =
    console.log("ExplorerController mounted");
    // Tell the extension that the view is ready to receive data.
    VSCode.postMessage({
      type: "init-view",
    });
  });

  //Window event handler
  function windowMessage(event) {
    const message = event.data; // The json data sent by the extension
    switch (message.type) {
      // TODO: Look into "state" optimization https://blog.kylekukshtel.com/game-data-editor-3
      case "init-global-states":
        suibaseData.globalStates.deserialize(message.data);
        console.log("init-global-states called", suibaseData.globalStates);
        $ui_selected_context = suibaseData.globalStates.uiSelectedContext;
        break;
      default:
        break;
    }
  }
</script>

<svelte:window on:message={windowMessage} />

<main>
  {#key ui_selected_context}
    <vscode-dropdown value="$ui_selected_context">
      {#each [...$all_contexts] as [key, mapping]}
        {#if key === $ui_selected_context}
          <vscode-option value={key} selected>
            {key},{mapping.context.ui_selector_name}
          </vscode-option>
        {:else}
          <vscode-option value={key}>
            {key},{mapping.context.ui_selector_name}
          </vscode-option>
        {/if}
      {/each}
    </vscode-dropdown>
  {/key}

  <select bind:value={$ui_selected_context}>
    {#each [...$all_contexts] as [key, mapping]}
      <option value={key}>
        {mapping.context.ui_selector_name}
      </option>
    {/each}
  </select>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
  }

  div {
    display: flex;
    flex-direction: row;
    justify-content: left;
    align-items: center;
    width: 400px;
  }

  div.workdir_row {
    display: flex;
    flex-direction: row;
    justify-content: left;
    align-items: center;
    width: 310px;
    gap: 10px;
    padding: 3px 0px;
    border: 1px solid gray;
    border-radius: 5px;
  }

  h2.workdir {
    margin: 0;
    font-size: 16px;
    width: 30px;
    text-align: left;
    padding-left: 5px;
  }

  h2.status {
    margin: 0;
    font-size: 9px;
    color: #666;
    width: 70px;
    text-align: right;
  }
</style>
