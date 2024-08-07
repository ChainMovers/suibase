import{_ as t}from"./plugin-vue_export-helper-DlAUqK2U.js";import{c as e,o,a}from"./app--bRAlwpO.js";const i={},s=a('<p>A notable Sui feature is its capability to handle fast &quot;Simple transaction&quot; at scale. These are for single-owner objects that do not require relatively more costly/slower consensus.</p><div class="hint-container caution"><p class="hint-container-title">Danger</p><p>Fast transaction have to be done with care to avoid equivocations. This can result in the dreaded &quot;quorum failure&quot; that locks your owned object until the end of an epoch. This guide should help you design your app to benefit from fast transactions AND remain reliable.</p></div><h2 id="don-t-do-this" tabindex="-1"><a class="header-anchor" href="#don-t-do-this"><span>Don&#39;t do this</span></a></h2><p>May be, the most important to understand is what not to do:</p><ul><li>Do not initiate multiple transaction with the same owned object at the same time.</li><li>Do not use the same coin with multiple simple transaction at the same time.</li></ul><h2 id="from-slow-to-fast" tabindex="-1"><a class="header-anchor" href="#from-slow-to-fast"><span>From Slow To Fast</span></a></h2><p>Todo Refer to example transforming a slow design into fast ones (think I saw one in the Sui repo?)</p><h2 id="distinct-coins" tabindex="-1"><a class="header-anchor" href="#distinct-coins"><span>Distinct Coins</span></a></h2><p>Todo Explain how distinct coin management is crucial to parallel processing.</p><h2 id="faucet" tabindex="-1"><a class="header-anchor" href="#faucet"><span>Faucet</span></a></h2><p>Todo Explain how the Sui faucet work as a design example.</p>',11),n=[s];function r(c,l){return o(),e("div",null,n)}const h=t(i,[["render",r],["__file","fast_transactions.html.vue"]]),u=JSON.parse(`{"path":"/cookbook/guides/fast_transactions.html","title":"Fast Transactions","lang":"en-US","frontmatter":{"title":"Fast Transactions","order":6,"contributors":true,"editLink":true,"description":"A notable Sui feature is its capability to handle fast \\"Simple transaction\\" at scale. These are for single-owner objects that do not require relatively more costly/slower consen...","head":[["meta",{"property":"og:url","content":"https://suibase.io/cookbook/guides/fast_transactions.html"}],["meta",{"property":"og:site_name","content":"suibase.io"}],["meta",{"property":"og:title","content":"Fast Transactions"}],["meta",{"property":"og:description","content":"A notable Sui feature is its capability to handle fast \\"Simple transaction\\" at scale. These are for single-owner objects that do not require relatively more costly/slower consen..."}],["meta",{"property":"og:type","content":"article"}],["meta",{"property":"og:locale","content":"en-US"}],["meta",{"property":"og:updated_time","content":"2024-08-06T04:52:57.000Z"}],["meta",{"property":"article:author","content":"suibase.io"}],["meta",{"property":"article:modified_time","content":"2024-08-06T04:52:57.000Z"}],["script",{"type":"application/ld+json"},"{\\"@context\\":\\"https://schema.org\\",\\"@type\\":\\"Article\\",\\"headline\\":\\"Fast Transactions\\",\\"image\\":[\\"\\"],\\"dateModified\\":\\"2024-08-06T04:52:57.000Z\\",\\"author\\":[{\\"@type\\":\\"Person\\",\\"name\\":\\"suibase.io\\",\\"url\\":\\"https://suibase.io\\"}]}"]]},"headers":[{"level":2,"title":"Don't do this","slug":"don-t-do-this","link":"#don-t-do-this","children":[]},{"level":2,"title":"From Slow To Fast","slug":"from-slow-to-fast","link":"#from-slow-to-fast","children":[]},{"level":2,"title":"Distinct Coins","slug":"distinct-coins","link":"#distinct-coins","children":[]},{"level":2,"title":"Faucet","slug":"faucet","link":"#faucet","children":[]}],"git":{"createdTime":1683694767000,"updatedTime":1722919977000,"contributors":[{"name":"mario4tier","email":"mario4tier@users.noreply.github.com","commits":2}]},"readingTime":{"minutes":0.59,"words":177},"filePathRelative":"cookbook/guides/fast_transactions.md","localizedDate":"May 10, 2023","autoDesc":true}`);export{h as comp,u as data};