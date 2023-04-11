import{_ as e,W as t,X as i,a1 as s}from"./framework-5e49288e.js";const n={},a=s(`<h1 id="references" tabindex="-1"><a class="header-anchor" href="#references" aria-hidden="true">#</a> References</h1><p>Sui-Base define a few conventions to coordinate among SDKs, apps and user.</p><h2 id="filesystem-path-convention" tabindex="-1"><a class="header-anchor" href="#filesystem-path-convention" aria-hidden="true">#</a> Filesystem Path Convention</h2><p>There are 6 &lt;WORKDIR&gt;:<br> active, localnet, devnet, testnet, mainnet and cargobin</p><p>Each &lt;WORKDIR&gt; has the following components:</p><table><thead><tr><th>Component</th><th>Purpose</th></tr></thead><tbody><tr><td>sui-exec</td><td>A script allowing any app to safely call the right sui client+config combination. Use it like you would use the &quot;sui&quot; client from Mysten Lab.</td></tr><tr><td>config</td><td>Directory with Mysten Lab files needed to run the sui client (client.yaml and sui.keystore).</td></tr><tr><td>sui-repo</td><td>A local repo of the Mysten lab sui code for building the client binary, but also for any apps to use the Rust SDK crates for compatibility.</td></tr><tr><td>published-data</td><td>Information about last package published from this &lt;WORKDIR&gt; using sui-base scripts. This can be retrieved through JSON files or through sui-base SDK helpers.</td></tr></tbody></table><p>Applications can expect the components to be always at these <strong>fix</strong> locations:</p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code> ~/
 └─ sui-base/
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
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p>??? abstract &quot;Official and Complete Path List&quot; ~/sui-base/workdirs/&lt;WORKDIR&gt;/sui-exec<br> ~/sui-base/workdirs/&lt;WORKDIR&gt;/config/client.yaml<br> ~/sui-base/workdirs/&lt;WORKDIR&gt;/config/sui.keystore<br> ~/sui-base/workdirs/&lt;WORKDIR&gt;/sui-repo/<br> ~/sui-base/workdirs/&lt;WORKDIR&gt;/published-data/&lt;PACKAGE_NAME&gt;/publish-output.json<br></p><p>TODO next:</p><ul><li>What is the &quot;active&quot; workdir?</li><li>What is the &quot;cargobin&quot; workdir?</li><li>How to use the sui-exec script?</li><li>How to use the publish-output.json?</li></ul><h2 id="sui-client-concurrency-limitation" tabindex="-1"><a class="header-anchor" href="#sui-client-concurrency-limitation" aria-hidden="true">#</a> Sui Client Concurrency Limitation</h2><p>Explain architecture limitation related to active-address, active-env, switch and such...</p>`,13),r=[a];function l(o,d){return t(),i("div",null,r)}const u=e(n,[["render",l],["__file","references.html.vue"]]);export{u as default};
