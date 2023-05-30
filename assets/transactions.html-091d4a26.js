import{_ as i}from"./plugin-vue_export-helper-c27b6911.js";import{r as l,o as u,c as p,d as o,w as a,e as r,a as n,b as s}from"./app-a39fc15f.js";const k={},d=r('<h2 id="facts" tabindex="-1"><a class="header-anchor" href="#facts" aria-hidden="true">#</a> Facts</h2><div class="hint-container tip"><p class="hint-container-title">Fact Sheet</p></div><p>Suggested subjects:</p><ul><li>How to transfer an object</li><li>How to transfer Sui</li><li>How to merge coins</li><li>How to publish a module</li></ul><h2 id="how-to-transfer-an-object" tabindex="-1"><a class="header-anchor" href="#how-to-transfer-an-object" aria-hidden="true">#</a> How to transfer an object</h2>',5),m=n("div",{class:"language-CLI line-numbers-mode","data-ext":"CLI"},[n("pre",{class:"language-CLI"},[n("code",null,`To be done. Add your contribution here.
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),b=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("To be done"),n("span",{class:"token punctuation"},"."),s(" Add your contribution here"),n("span",{class:"token punctuation"},"."),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),v=n("div",{class:"language-typescript line-numbers-mode","data-ext":"ts"},[n("pre",{class:"language-typescript"},[n("code",null,[n("span",{class:"token keyword"},"import"),s(),n("span",{class:"token punctuation"},"{"),s(`
    Ed25519Keypair`),n("span",{class:"token punctuation"},","),s(`
    Connection`),n("span",{class:"token punctuation"},","),s(`
    JsonRpcProvider`),n("span",{class:"token punctuation"},","),s(`
    RawSigner`),n("span",{class:"token punctuation"},","),s(`
    TransactionBlock`),n("span",{class:"token punctuation"},","),s(`
  `),n("span",{class:"token punctuation"},"}"),s(),n("span",{class:"token keyword"},"from"),s(),n("span",{class:"token string"},'"@mysten/sui.js"'),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Set a provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" connection "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Connection"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    fullnode`),n("span",{class:"token operator"},":"),s(),n("span",{class:"token string"},'"http://127.0.0.1:9000"'),n("span",{class:"token punctuation"},","),s(`
  `),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Generate a new Ed25519 Keypair"),s(`
`),n("span",{class:"token keyword"},"const"),s(" keypair "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Ed25519Keypair"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`
  
`),n("span",{class:"token comment"},"// Connect to the provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" provider "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"JsonRpcProvider"),n("span",{class:"token punctuation"},"("),s("connection"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate RawSigner object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" signer "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"RawSigner"),n("span",{class:"token punctuation"},"("),s("keypair"),n("span",{class:"token punctuation"},","),s(" provider"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate TransactionBlock object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" tx "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"TransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Build the transfer object"),s(`
tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"transferObjects"),n("span",{class:"token punctuation"},"("),s(`
`),n("span",{class:"token punctuation"},"["),s(`
    tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"object"),n("span",{class:"token punctuation"},"("),s(`
        `),n("span",{class:"token string"},"'0xe19739da1a701eadc21683c5b127e62b553e833e8a15a4f292f4f48b4afea3f2'"),n("span",{class:"token punctuation"},","),s(`
    `),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},"]"),n("span",{class:"token punctuation"},","),s(`
    tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"pure"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string"},"'0x1d20dcdb2bca4f508ea9613994683eb4e76e9c4ed371169677c1be02aaf0b12a'"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Perform the object transfer"),s(`
`),n("span",{class:"token keyword"},"const"),s(" result "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"await"),s(" signer"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"signAndExecuteTransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    transactionBlock`),n("span",{class:"token operator"},":"),s(" tx"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Print the output"),s(`
`),n("span",{class:"token builtin"},"console"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"log"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(" result "),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),w=n("h2",{id:"how-to-transfer-sui",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#how-to-transfer-sui","aria-hidden":"true"},"#"),s(" How to transfer Sui")],-1),y=n("div",{class:"language-CLI line-numbers-mode","data-ext":"CLI"},[n("pre",{class:"language-CLI"},[n("code",null,`To be done. Add your contribution here.
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),h=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("To be done"),n("span",{class:"token punctuation"},"."),s(" Add your contribution here"),n("span",{class:"token punctuation"},"."),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),g=n("div",{class:"language-typescript line-numbers-mode","data-ext":"ts"},[n("pre",{class:"language-typescript"},[n("code",null,[n("span",{class:"token keyword"},"import"),s(),n("span",{class:"token punctuation"},"{"),s(`
    Ed25519Keypair`),n("span",{class:"token punctuation"},","),s(`
    Connection`),n("span",{class:"token punctuation"},","),s(`
    JsonRpcProvider`),n("span",{class:"token punctuation"},","),s(`
    RawSigner`),n("span",{class:"token punctuation"},","),s(`
    TransactionBlock`),n("span",{class:"token punctuation"},","),s(`
  `),n("span",{class:"token punctuation"},"}"),s(),n("span",{class:"token keyword"},"from"),s(),n("span",{class:"token string"},'"@mysten/sui.js"'),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Generate a new Keypair"),s(`
`),n("span",{class:"token keyword"},"const"),s(" keypair "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Ed25519Keypair"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Set a provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" connection "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Connection"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    fullnode`),n("span",{class:"token operator"},":"),s(),n("span",{class:"token string"},'"http://127.0.0.1:9000"'),n("span",{class:"token punctuation"},","),s(`
  `),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Connect to provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" provider "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"JsonRpcProvider"),n("span",{class:"token punctuation"},"("),s("connection"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate RawSigner object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" signer "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"RawSigner"),n("span",{class:"token punctuation"},"("),s("keypair"),n("span",{class:"token punctuation"},","),s(" provider"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate TransactionBlock object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" tx "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"TransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Set 1 Sui to be sent"),s(`
`),n("span",{class:"token keyword"},"const"),s(),n("span",{class:"token punctuation"},"["),s("coin"),n("span",{class:"token punctuation"},"]"),s(),n("span",{class:"token operator"},"="),s(" tx"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"splitCoins"),n("span",{class:"token punctuation"},"("),s("tx"),n("span",{class:"token punctuation"},"."),s("gas"),n("span",{class:"token punctuation"},","),s(),n("span",{class:"token punctuation"},"["),s("tx"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"pure"),n("span",{class:"token punctuation"},"("),n("span",{class:"token number"},"1_000_000_000"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},"]"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"transferObjects"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"["),s("coin"),n("span",{class:"token punctuation"},"]"),n("span",{class:"token punctuation"},","),s(" tx"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"pure"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string"},'"0x8bab471b0b2e69ac5051c58bbbf81159c4c9d42bf7a58d4f795ecfb12c968506"'),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Perform SUI transfer"),s(`
`),n("span",{class:"token keyword"},"const"),s(" result "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"await"),s(" signer"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"signAndExecuteTransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    transactionBlock`),n("span",{class:"token operator"},":"),s(" tx"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Print output"),s(`
`),n("span",{class:"token builtin"},"console"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"log"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(" result "),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),f=n("h2",{id:"how-to-merge-coins",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#how-to-merge-coins","aria-hidden":"true"},"#"),s(" How to merge coins")],-1),_=n("div",{class:"language-CLI line-numbers-mode","data-ext":"CLI"},[n("pre",{class:"language-CLI"},[n("code",null,`To be done. Add your contribution here.
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),x=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("To be done"),n("span",{class:"token punctuation"},"."),s(" Add your contribution here"),n("span",{class:"token punctuation"},"."),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),C=n("div",{class:"language-typescript line-numbers-mode","data-ext":"ts"},[n("pre",{class:"language-typescript"},[n("code",null,[n("span",{class:"token keyword"},"import"),s(),n("span",{class:"token punctuation"},"{"),s(`
    Ed25519Keypair`),n("span",{class:"token punctuation"},","),s(`
    Connection`),n("span",{class:"token punctuation"},","),s(`
    JsonRpcProvider`),n("span",{class:"token punctuation"},","),s(`
    RawSigner`),n("span",{class:"token punctuation"},","),s(`
    TransactionBlock`),n("span",{class:"token punctuation"},","),s(`
  `),n("span",{class:"token punctuation"},"}"),s(),n("span",{class:"token keyword"},"from"),s(),n("span",{class:"token string"},'"@mysten/sui.js"'),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Generate a new Keypair"),s(`
`),n("span",{class:"token keyword"},"const"),s(" keypair "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Ed25519Keypair"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Set a provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" connection "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"Connection"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    fullnode`),n("span",{class:"token operator"},":"),s(),n("span",{class:"token string"},'"http://127.0.0.1:9000"'),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Connect to provider"),s(`
`),n("span",{class:"token keyword"},"const"),s(" provider "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"JsonRpcProvider"),n("span",{class:"token punctuation"},"("),s("connection"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate RawSigner object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" signer "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"RawSigner"),n("span",{class:"token punctuation"},"("),s("keypair"),n("span",{class:"token punctuation"},","),s(" provider"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Instantiate TransactionBlock object"),s(`
`),n("span",{class:"token keyword"},"const"),s(" tx "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"new"),s(),n("span",{class:"token class-name"},"TransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Build merge transaction"),s(`
tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"mergeCoins"),n("span",{class:"token punctuation"},"("),s(`
        tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"object"),n("span",{class:"token punctuation"},"("),s(`
            `),n("span",{class:"token string"},"'0x5406c80f34fb770d9cd4226ddc6208164d3c52dbccdf84a6805aa66c0ef8f01f'"),n("span",{class:"token punctuation"},","),s(`
        `),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s(`
    `),n("span",{class:"token punctuation"},"["),s(`
        tx`),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"object"),n("span",{class:"token punctuation"},"("),s(`
            `),n("span",{class:"token string"},"'0x918af8a3580b1b9562c0fddaf102b482d51a5806f4485b831aca6feec04f7c3f'"),n("span",{class:"token punctuation"},","),s(`
        `),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s(`
    `),n("span",{class:"token punctuation"},"]"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Perform the merge"),s(`
`),n("span",{class:"token keyword"},"const"),s(" result "),n("span",{class:"token operator"},"="),s(),n("span",{class:"token keyword"},"await"),s(" signer"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"signAndExecuteTransactionBlock"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(`
    transactionBlock`),n("span",{class:"token operator"},":"),s(" tx"),n("span",{class:"token punctuation"},","),s(`
`),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`),n("span",{class:"token comment"},"// Print the output"),s(`
`),n("span",{class:"token builtin"},"console"),n("span",{class:"token punctuation"},"."),n("span",{class:"token function"},"log"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},"{"),s(" result "),n("span",{class:"token punctuation"},"}"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},";"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1);function A(T,S){const c=l("CodeTabs");return u(),p("div",null,[d,o(c,{id:"33",data:[{id:"CLI"},{id:"Python"},{id:"TS"}],active:0},{title0:a(({value:e,isActive:t})=>[s("CLI")]),title1:a(({value:e,isActive:t})=>[s("Python")]),title2:a(({value:e,isActive:t})=>[s("TS")]),tab0:a(({value:e,isActive:t})=>[m]),tab1:a(({value:e,isActive:t})=>[b]),tab2:a(({value:e,isActive:t})=>[v]),_:1}),w,o(c,{id:"47",data:[{id:"CLI"},{id:"Python"},{id:"TS"}],active:0},{title0:a(({value:e,isActive:t})=>[s("CLI")]),title1:a(({value:e,isActive:t})=>[s("Python")]),title2:a(({value:e,isActive:t})=>[s("TS")]),tab0:a(({value:e,isActive:t})=>[y]),tab1:a(({value:e,isActive:t})=>[h]),tab2:a(({value:e,isActive:t})=>[g]),_:1}),f,o(c,{id:"61",data:[{id:"CLI"},{id:"Python"},{id:"TS"}],active:0},{title0:a(({value:e,isActive:t})=>[s("CLI")]),title1:a(({value:e,isActive:t})=>[s("Python")]),title2:a(({value:e,isActive:t})=>[s("TS")]),tab0:a(({value:e,isActive:t})=>[_]),tab1:a(({value:e,isActive:t})=>[x]),tab2:a(({value:e,isActive:t})=>[C]),_:1})])}const B=i(k,[["render",A],["__file","transactions.html.vue"]]);export{B as default};
