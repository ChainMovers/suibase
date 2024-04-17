
  //import { VSCode } from "../lib/VSCode";
  //import { SuibaseData } from "../common/SuibaseData";

  export const ExplorerController = () => {

  // Data exchanged with the extension.
  //let suibaseData: SuibaseData = SuibaseData.getInstance();

  return (
    <>Explorer Controller
    </>
  );
  /*
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
</main>*/
  }

