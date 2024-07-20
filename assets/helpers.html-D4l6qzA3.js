import{_ as p}from"./plugin-vue_export-helper-DlAUqK2U.js";import{r as o,o as u,c as d,a as e,d as s,b as i,w as n,e as m}from"./app-Ct3oRgcQ.js";const h={},b=e("br",null,null,-1),k=e("br",null,null,-1),v=e("br",null,null,-1),g=m('<h2 id="what-is-a-suibase-helper" tabindex="-1"><a class="header-anchor" href="#what-is-a-suibase-helper"><span>What is a Suibase Helper?</span></a></h2><p>An API providing information to accelerate the development and testing of Sui apps.</p><p>Your app get access to:</p><ul><li>Package ID of most recently published modules (can query by name).</li><li>IDs of the shared objects created on last publish of your module.</li><li>active client address (can also query by alias).</li><li>A healthy RPC URL for a specific network (e.g. devnet).</li><li>Various utility functions to help automating development.</li></ul><p><strong>How it works?</strong><br> The magic happens when you do a workdir &quot;publish&quot; command (e.g. <code>testnet publish</code>). This is a drop-in replacement of the Sui binary approach (e.g. <code>sui publish</code>) and the same parameters can be specified.</p><p>The Suibase command calls the proper Mysten Labs Sui client version matching the network. It adds parameters to save the output in a JSON file. The data is copied in the Suibase workdir structure, and becomes accessible to your apps through an Helper API.</p><h3 id="example-1-what-is-the-active-client-address-for-localnet" tabindex="-1"><a class="header-anchor" href="#example-1-what-is-the-active-client-address-for-localnet"><span>Example 1: What is the active client address for localnet?</span></a></h3>',7),f=e("div",{class:"language-python line-numbers-mode","data-ext":"py","data-title":"py"},[e("pre",{class:"language-python"},[e("code",null,[s("    "),e("span",{class:"token keyword"},"import"),s(" suibase"),e("span",{class:"token punctuation"},";"),s(`

    `),e("span",{class:"token comment"},"# Create suibase helper."),s(`
    sbh `),e("span",{class:"token operator"},"="),s(" suibase"),e("span",{class:"token punctuation"},"."),s("Helper"),e("span",{class:"token punctuation"},"("),e("span",{class:"token punctuation"},")"),s(`
    `),e("span",{class:"token keyword"},"if"),s(),e("span",{class:"token keyword"},"not"),s(" sbh"),e("span",{class:"token punctuation"},"."),s("is_installed"),e("span",{class:"token punctuation"},"("),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},":"),s(`
        `),e("span",{class:"token keyword"},"print"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"suibase is not installed. Please do ~/suibase/install first."'),e("span",{class:"token punctuation"},")"),s(`
        exit`),e("span",{class:"token punctuation"},"("),e("span",{class:"token number"},"1"),e("span",{class:"token punctuation"},")"),s(`

    `),e("span",{class:"token comment"},"# Select a workdir."),s(`
    sbh`),e("span",{class:"token punctuation"},"."),s("select_workdir"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"localnet"'),e("span",{class:"token punctuation"},")"),s(`

    `),e("span",{class:"token comment"},'# Print the active address, same as "sui client active-address"'),s(`
    active_address `),e("span",{class:"token operator"},"="),s(" sbh"),e("span",{class:"token punctuation"},"."),s("client_address"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"active"'),e("span",{class:"token punctuation"},")"),s(`
    `),e("span",{class:"token keyword"},"print"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string-interpolation"},[e("span",{class:"token string"},'f"Active address: '),e("span",{class:"token interpolation"},[e("span",{class:"token punctuation"},"{"),s(" active_address "),e("span",{class:"token punctuation"},"}")]),e("span",{class:"token string"},'"')]),e("span",{class:"token punctuation"},")"),s(`

    `),e("span",{class:"token comment"},'# Suibase supports more than just "active"...'),s(`
    `),e("span",{class:"token comment"},"#"),s(`
    `),e("span",{class:"token comment"},"# localnet has *always* at least 15 named addresses for deterministic test setups."),s(`
    `),e("span",{class:"token comment"},"#"),s(`
    `),e("span",{class:"token comment"},"# Get one of these address using its alias."),s(`
    test_address_1 `),e("span",{class:"token operator"},"="),s(" sbh"),e("span",{class:"token punctuation"},"."),s("client_address"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"sb-1-ed25519"'),e("span",{class:"token punctuation"},")"),s(`
    `),e("span",{class:"token keyword"},"print"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string-interpolation"},[e("span",{class:"token string"},'f"Test address 1 type ed25519: '),e("span",{class:"token interpolation"},[e("span",{class:"token punctuation"},"{"),s(" test_address_1 "),e("span",{class:"token punctuation"},"}")]),e("span",{class:"token string"},'"')]),e("span",{class:"token punctuation"},")"),s(`

    `),e("span",{class:"token comment"},"######## Console output #####"),s(`
    `),e("span",{class:"token comment"},"# Active address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462"),s(`
    `),e("span",{class:"token comment"},"# Test address 1 type ed25519: 0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b"),s(`
    `),e("span",{class:"token comment"},"#############################"),s(`
`)])]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"})])],-1),y=e("div",{class:"language-rust line-numbers-mode","data-ext":"rs","data-title":"rs"},[e("pre",{class:"language-rust"},[e("code",null,[s("  "),e("span",{class:"token keyword"},"use"),s(),e("span",{class:"token namespace"},[s("suibase"),e("span",{class:"token punctuation"},"::")]),e("span",{class:"token class-name"},"Helper"),e("span",{class:"token punctuation"},";"),s(`
  `),e("span",{class:"token keyword"},"fn"),s(),e("span",{class:"token function-definition function"},"main"),e("span",{class:"token punctuation"},"("),e("span",{class:"token punctuation"},")"),s(),e("span",{class:"token punctuation"},"{"),s(`
    `),e("span",{class:"token comment"},"// Create a Suibase helper API."),s(`
    `),e("span",{class:"token keyword"},"let"),s(" sbh "),e("span",{class:"token operator"},"="),s(),e("span",{class:"token class-name"},"Helper"),e("span",{class:"token punctuation"},"::"),e("span",{class:"token function"},"new"),e("span",{class:"token punctuation"},"("),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},";"),s(`

    `),e("span",{class:"token keyword"},"if"),s(" sbh"),e("span",{class:"token punctuation"},"."),e("span",{class:"token function"},"is_installed"),e("span",{class:"token punctuation"},"("),e("span",{class:"token punctuation"},")"),e("span",{class:"token operator"},"?"),s(),e("span",{class:"token punctuation"},"{"),s(`
       `),e("span",{class:"token comment"},"// Select the localnet workdir."),s(`
       sbh`),e("span",{class:"token punctuation"},"."),e("span",{class:"token function"},"select_workdir"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"localnet"'),e("span",{class:"token punctuation"},")"),e("span",{class:"token operator"},"?"),e("span",{class:"token punctuation"},";"),s(`

       `),e("span",{class:"token comment"},'// Print the active address, same as "sui client active-address"'),s(`
       `),e("span",{class:"token macro property"},"println!"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"Active address: {}"'),e("span",{class:"token punctuation"},","),s(" sbh"),e("span",{class:"token punctuation"},"."),e("span",{class:"token function"},"client_address"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"active"'),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},";"),s(`

       `),e("span",{class:"token comment"},'// Suibase supports more than just "active"...'),s(`
       `),e("span",{class:"token comment"},"//"),s(`
       `),e("span",{class:"token comment"},"// localnet has *always* at least 15 named addresses for deterministic test setups."),s(`
       `),e("span",{class:"token comment"},"//"),s(`
       `),e("span",{class:"token comment"},"// Get one of these address using its alias."),s(`
       `),e("span",{class:"token keyword"},"let"),s(" test_address "),e("span",{class:"token operator"},"="),s(" sbh"),e("span",{class:"token punctuation"},"."),e("span",{class:"token function"},"client_address"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"sb-1-ed25519"'),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},";"),s(`
       `),e("span",{class:"token macro property"},"println!"),e("span",{class:"token punctuation"},"("),e("span",{class:"token string"},'"Test address 1 type ed25519: {}"'),e("span",{class:"token punctuation"},","),s(" test_address "),e("span",{class:"token punctuation"},")"),e("span",{class:"token punctuation"},";"),s(`
    `),e("span",{class:"token punctuation"},"}"),s(`
  `),e("span",{class:"token punctuation"},"}"),s(`

  `),e("span",{class:"token comment"},"//Console output:"),s(`
  `),e("span",{class:"token comment"},"//Active address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462"),s(`
  `),e("span",{class:"token comment"},"//Test address 1 type ed25519: 0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b"),s(`

`)])]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"}),e("div",{class:"line-number"})])],-1),_=e("h4",{id:"example-2-what-is-my-last-published-package-id-on-devnet",tabindex:"-1"},[e("a",{class:"header-anchor",href:"#example-2-what-is-my-last-published-package-id-on-devnet"},[e("span",null,"Example 2: What is my last published package ID on devnet?")])],-1),w=e("div",{class:"language-python line-numbers-mode","data-ext":"py","data-title":"py"},[e("pre",{class:"language-python"},[e("code",null,`TODO
`)]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"})])],-1),x=e("div",{class:"language-rust line-numbers-mode","data-ext":"rs","data-title":"rs"},[e("pre",{class:"language-rust"},[e("code",null,[e("span",{class:"token constant"},"TODO"),s(`
`)])]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"})])],-1),A=e("h4",{id:"example-3-which-url-should-be-used-right-now-for-testnet",tabindex:"-1"},[e("a",{class:"header-anchor",href:"#example-3-which-url-should-be-used-right-now-for-testnet"},[e("span",null,"Example 3: Which URL should be used right now for testnet?")])],-1),S=e("p",null,"Suibase monitor RPC health of multiple servers and return the best URL to use.",-1),T=e("div",{class:"language-python line-numbers-mode","data-ext":"py","data-title":"py"},[e("pre",{class:"language-python"},[e("code",null,`TODO
`)]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"})])],-1),P=e("div",{class:"language-rust line-numbers-mode","data-ext":"rs","data-title":"rs"},[e("pre",{class:"language-rust"},[e("code",null,[e("span",{class:"token constant"},"TODO"),s(`
`)])]),e("div",{class:"line-numbers","aria-hidden":"true"},[e("div",{class:"line-number"})])],-1);function H(R,O){const c=o("iconify-icon"),r=o("RouteLink"),l=o("CodeTabs");return u(),d("div",null,[e("p",null,[s("This page is an introduction. When ready, check the following for language specific docs:"),b,i(r,{to:"/rust/helper.html"},{default:n(()=>[i(c,{class:"font-icon icon",icon:"marketeq:curve-arrow-right"}),s(" Rust Helper")]),_:1}),k,i(r,{to:"/python/helper.html"},{default:n(()=>[i(c,{class:"font-icon icon",icon:"marketeq:curve-arrow-right"}),s(" Python Helper")]),_:1}),v]),g,i(l,{id:"48",data:[{id:"Python"},{id:"Rust"}],active:0},{title0:n(({value:a,isActive:t})=>[s("Python")]),title1:n(({value:a,isActive:t})=>[s("Rust")]),tab0:n(({value:a,isActive:t})=>[f]),tab1:n(({value:a,isActive:t})=>[y]),_:1}),_,i(l,{id:"59",data:[{id:"Python"},{id:"Rust"}],active:0},{title0:n(({value:a,isActive:t})=>[s("Python")]),title1:n(({value:a,isActive:t})=>[s("Rust")]),tab0:n(({value:a,isActive:t})=>[w]),tab1:n(({value:a,isActive:t})=>[x]),_:1}),A,S,i(l,{id:"73",data:[{id:"Python"},{id:"Rust"}],active:0},{title0:n(({value:a,isActive:t})=>[s("Python")]),title1:n(({value:a,isActive:t})=>[s("Rust")]),tab0:n(({value:a,isActive:t})=>[T]),tab1:n(({value:a,isActive:t})=>[P]),_:1})])}const D=p(h,[["render",H],["__file","helpers.html.vue"]]),I=JSON.parse('{"path":"/helpers.html","title":"Suibase Helpers Overview","lang":"en-US","frontmatter":{"title":"Suibase Helpers Overview","description":"This page is an introduction. When ready, check the following for language specific docs: What is a Suibase Helper? An API providing information to accelerate the development an...","head":[["meta",{"property":"og:url","content":"https://suibase.io/helpers.html"}],["meta",{"property":"og:site_name","content":"suibase.io"}],["meta",{"property":"og:title","content":"Suibase Helpers Overview"}],["meta",{"property":"og:description","content":"This page is an introduction. When ready, check the following for language specific docs: What is a Suibase Helper? An API providing information to accelerate the development an..."}],["meta",{"property":"og:type","content":"article"}],["meta",{"property":"og:locale","content":"en-US"}],["meta",{"property":"og:updated_time","content":"2024-04-25T01:29:57.000Z"}],["meta",{"property":"article:author","content":"suibase.io"}],["meta",{"property":"article:modified_time","content":"2024-04-25T01:29:57.000Z"}],["script",{"type":"application/ld+json"},"{\\"@context\\":\\"https://schema.org\\",\\"@type\\":\\"Article\\",\\"headline\\":\\"Suibase Helpers Overview\\",\\"image\\":[\\"\\"],\\"dateModified\\":\\"2024-04-25T01:29:57.000Z\\",\\"author\\":[{\\"@type\\":\\"Person\\",\\"name\\":\\"suibase.io\\",\\"url\\":\\"https://suibase.io\\"}]}"]]},"headers":[{"level":2,"title":"What is a Suibase Helper?","slug":"what-is-a-suibase-helper","link":"#what-is-a-suibase-helper","children":[{"level":3,"title":"Example 1: What is the active client address for localnet?","slug":"example-1-what-is-the-active-client-address-for-localnet","link":"#example-1-what-is-the-active-client-address-for-localnet","children":[]}]}],"git":{"createdTime":1679706144000,"updatedTime":1714008597000,"contributors":[{"name":"mario4tier","email":"mario4tier@users.noreply.github.com","commits":7}]},"readingTime":{"minutes":1.58,"words":475},"filePathRelative":"helpers.md","localizedDate":"March 25, 2023","autoDesc":true}');export{D as comp,I as data};