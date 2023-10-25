<script lang="ts">
  import { VSCode } from "../lib/VSCode";
  import { SuibaseJSONStorage } from "../common/SuibaseJSONStorage";
</script>

<main>
  <ul class="no-bullets">
    <li>
      2023-10-25 14:46:19.031 [info] > git show --textconv
      :typescript/vscode-extension/webview-ui/src/components/ConsoleController.svelte [21ms]
    </li>

    <li>
      2023-10-25 14:46:19.032 [info] > git ls-files --stage --
      /home/olet/suibase/typescript/vscode-extension/webview-ui/src/components/ConsoleController.svelte [1ms]
    </li>

    <li>2023-10-25 14:53:26.114 [info] > git fetch [536ms]</li>

    <li>2023-10-25 14:53:26.145 [info] > git config --get commit.template [1ms]</li>

    <li>
      2023-10-25 14:53:26.165 [info] > git for-each-ref
      --format=%(refname)%00%(upstream:short)%00%(objectname)%00%(upstream:track)%00%(upstream:remotename)%00%(upstream:remoteref)
      refs/heads/main refs/remotes/main [1ms]
    </li>

    <li>2023-10-25 14:53:26.181 [info] > git status -z -uall [3ms]</li>
    <li>
      2023-10-25 14:53:27.331 [info] > git ls-tree -l HEAD --
      /home/olet/suibase/typescript/vscode-extension/src/panels/BasePanel.ts [18ms]
    </li>
    <li>
      2023-10-25 14:53:27.332 [info] > git ls-files --stage --
      /home/olet/suibase/typescript/vscode-extension/webview-ui/src/components/ConsoleController.svelte [1ms]
    </li>
  </ul>
</main>

<style>
  /* Hide everything above this component. */
  /*:global(#smui-app),
  :global(body),
  :global(html) {
    display: block !important;
    height: auto !important;
    width: auto !important;
    position: static !important;
  }*/

  main {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
  }

  ul.no-bullets {
    list-style-type: none; /* Remove bullets */
    padding: 0; /* Remove padding */
    margin: 0; /* Remove margins */
  }
</style>
