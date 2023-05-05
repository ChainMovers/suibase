import{_ as e,Y as i,Z as n,a3 as t}from"./framework-1aca60a5.js";const s={},a=t(`<h1 id="references" tabindex="-1"><a class="header-anchor" href="#references" aria-hidden="true">#</a> References</h1><p>Suibase define a few conventions to coordinate among SDKs, apps and user.</p><h2 id="filesystem-path-convention" tabindex="-1"><a class="header-anchor" href="#filesystem-path-convention" aria-hidden="true">#</a> Filesystem Path Convention</h2><p>There are 6 &lt;WORKDIR&gt;:<br> active, localnet, devnet, testnet, mainnet and cargobin</p><p>Each &lt;WORKDIR&gt; has the following components:</p><table><thead><tr><th>Component</th><th>Purpose</th></tr></thead><tbody><tr><td>sui-exec</td><td>A script allowing any app to safely call the right sui client+config combination. Use it like you would use the &quot;sui&quot; client from Mysten Lab.</td></tr><tr><td>config</td><td>Directory with Mysten Lab files needed to run the sui client (client.yaml and sui.keystore).</td></tr><tr><td>sui-repo</td><td>A local repo of the Mysten lab sui code for building the client binary, but also for any apps to use the Rust SDK crates for compatibility.</td></tr><tr><td>published-data</td><td>Information about last package published from this &lt;WORKDIR&gt; using suibase scripts. This can be retrieved through JSON files or through suibase SDK helpers.</td></tr></tbody></table><p>Applications can expect the components to be always at these <strong>fix</strong> locations:</p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code> ~/
 └─ suibase/
      └─ workdirs/
           └─ &lt;WORKDIR&gt;/
                 ├── sui-exec
                 │
                 ├── config
                 │      ├── client.yaml
                 │      └── sui.keystore
                 │
                 ├── sui-repo
                 │      ├── crates/
                 │      ├── target/
                 │      └── ... complete sui repo (debug built) ...
                 │
                 └── published-data
                        └─ &lt;package name&gt;
                                └─ publish-output.json

::: details Official and Complete Path List
    ~/suibase/workdirs/&lt;WORKDIR\\&gt;/sui-exec&lt;br&gt;
    ~/suibase/workdirs/&lt;WORKDIR\\&gt;/config/client.yaml&lt;br&gt;
    ~/suibase/workdirs/&lt;WORKDIR\\&gt;/config/sui.keystore&lt;br&gt;
    ~/suibase/workdirs/&lt;WORKDIR\\&gt;/sui-repo/&lt;br&gt;
    ~/suibase/workdirs/&lt;WORKDIR\\&gt;/published-data/&lt;PACKAGE_NAME\\&gt;/publish-output.json&lt;br&gt;


TODO next:

- What is the &quot;active&quot; workdir?
- What is the &quot;cargobin&quot; workdir?
- How to use the sui-exec script?
- How to use the publish-output.json?

## Sui Client Concurrency Limitation
Explain architecture limitation related to active-address, active-env, switch and such...

</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div>`,8),l=[a];function d(r,c){return i(),n("div",null,l)}const u=e(s,[["render",d],["__file","references.html.vue"]]);export{u as default};
