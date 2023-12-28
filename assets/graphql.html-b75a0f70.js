import{_ as o}from"./plugin-vue_export-helper-c27b6911.js";import{r as c,o as u,c as r,d as l,w as e,e as p,a as n,b as s}from"./app-d07328e8.js";const d={},m=p('<h2 id="facts" tabindex="-1"><a class="header-anchor" href="#facts" aria-hidden="true">#</a> Facts</h2><div class="hint-container tip"><p class="hint-container-title">Fact Sheet</p><ul><li>Sui GraphQL RPC is currently in <strong><em>beta</em></strong></li><li>Sui GraphQL RPC beta operates on a snapshot of data, it is not maintaining beyond: <ul><li>testnet data timestamp: &quot;2023-12-16T19:07:30.993Z&quot;</li><li>mainnet data timestamp: &quot;2023-11-21T22:03:27.667Z&quot;</li><li>devnet not supported</li></ul></li><li>Sui GraphQL RPC will eventually <em>replace</em> the JSON RPC</li><li>PySui support for Sui GraphQL RPC: <ul><li>Release 0.50.0 includes an &#39;experimental&#39; implementation, subject to change</li><li>Provides Synchronous and asynchronous GraphQL clients</li><li>Only &#39;read&#39; queries are supported at the time of this writing</li><li>Introduces <code>QueryNodes</code> that are the equivalent to pysui <code>Builders</code></li><li>Parity of QueryNodes to Builders is ongoing</li><li>Exposes ability for developers to write their own GraphQL queries</li><li>Must point to either <code>testnet</code> or <code>mainnet</code></li></ul></li></ul></div><h2 id="generating-graphql-schema" tabindex="-1"><a class="header-anchor" href="#generating-graphql-schema" aria-hidden="true">#</a> Generating GraphQL schema</h2>',3),k=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[s("NA at this "),n("span",{class:"token function"},"time"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),v=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Dump Sui GraphQL Schema."""'),s(`
        `),n("span",{class:"token comment"},"# Initialize synchronous client (must be mainnet or testnet)"),s(`
        client_init `),n("span",{class:"token operator"},"="),s(" SuiGQLClient"),n("span",{class:"token punctuation"},"("),s("config"),n("span",{class:"token operator"},"="),s("SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s("write_schema"),n("span",{class:"token operator"},"="),n("span",{class:"token boolean"},"True"),n("span",{class:"token punctuation"},")"),s(`

        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string"},'"Schema dumped to: `latest_schemaVERSION.graqhql`"'),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token keyword"},"if"),s(" __name__ "),n("span",{class:"token operator"},"=="),s(),n("span",{class:"token string"},'"__main__"'),n("span",{class:"token punctuation"},":"),s(`
        main`),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),b=n("h2",{id:"query-example-1",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#query-example-1","aria-hidden":"true"},"#"),s(" Query example 1")],-1),h=n("p",null,[s("For pysui there are 2 comon ways to create a query. This demonstrates "),n("strong",null,[n("em",null,"using QueryNodes (predefined queries as part of pysui SDK)")])],-1),g=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[s("NA at this "),n("span",{class:"token function"},"time"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),y=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token comment"},"#"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Query using predefined pysui QueryNode."""'),s(`

    `),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"import"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("pgql_query "),n("span",{class:"token keyword"},"as"),s(` qn
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),s("client"),n("span",{class:"token punctuation"},":"),s(" SuiGQLClient"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Fetch 0x2::sui::SUI (default) for owner."""'),s(`
        `),n("span",{class:"token comment"},"# GetCoins defaults to '0x2::sui::SUI' coin type so great for owners gas listing"),s(`
        `),n("span",{class:"token comment"},"# Replace <ADDRESS_STRING> with a valid testnet or mainnet address"),s(`
        qres `),n("span",{class:"token operator"},"="),s(" client"),n("span",{class:"token punctuation"},"."),s("execute_query"),n("span",{class:"token punctuation"},"("),s(`
            with_query_node`),n("span",{class:"token operator"},"="),s("qn"),n("span",{class:"token punctuation"},"."),s("GetCoins"),n("span",{class:"token punctuation"},"("),s(`
                owner`),n("span",{class:"token operator"},"="),n("span",{class:"token string"},'"<ADDRESS_STRING>"'),s(`
            `),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token comment"},"# Results are mapped to dataclasses/dataclasses-json"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),s("qres"),n("span",{class:"token punctuation"},"."),s("to_json"),n("span",{class:"token punctuation"},"("),s("indent"),n("span",{class:"token operator"},"="),n("span",{class:"token number"},"2"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token keyword"},"if"),s(" __name__ "),n("span",{class:"token operator"},"=="),s(),n("span",{class:"token string"},'"__main__"'),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token comment"},"# Initialize synchronous client (must be mainnet or testnet)"),s(`
        client_init `),n("span",{class:"token operator"},"="),s(" SuiGQLClient"),n("span",{class:"token punctuation"},"("),s("config"),n("span",{class:"token operator"},"="),s("SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s("write_schema"),n("span",{class:"token operator"},"="),n("span",{class:"token boolean"},"False"),n("span",{class:"token punctuation"},")"),s(`
        main`),n("span",{class:"token punctuation"},"("),s("client_init"),n("span",{class:"token punctuation"},")"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),_=n("h2",{id:"query-example-2",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#query-example-2","aria-hidden":"true"},"#"),s(" Query example 2")],-1),f=n("p",null,[s("For pysui there are 2 comon ways to create a query. This demonstrates "),n("strong",null,[n("em",null,"using a query string")])],-1),q=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[s("NA at this "),n("span",{class:"token function"},"time"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),w=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token comment"},"#"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Query using a query string."""'),s(`

    `),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),s("client"),n("span",{class:"token punctuation"},":"),s(" SuiGQLClient"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Configuration and protocol information."""'),s(`
        _QUERY `),n("span",{class:"token operator"},"="),s(),n("span",{class:"token triple-quoted-string string"},`"""
            query {
                chainIdentifier
                checkpointConnection (last: 1) {
                    nodes {
                        sequenceNumber
                        timestamp
                    }
                }
                serviceConfig {
                    enabledFeatures
                    maxQueryDepth
                    maxQueryNodes
                    maxDbQueryCost
                    maxPageSize
                    requestTimeoutMs
                    maxQueryPayloadSize
                }
            protocolConfig {
                protocolVersion
                configs {
                    key
                    value
                }
                featureFlags {
                    key
                    value
                }
                }
            }
        """`),s(`
        qres `),n("span",{class:"token operator"},"="),s(" client"),n("span",{class:"token punctuation"},"."),s("execute_query"),n("span",{class:"token punctuation"},"("),s("with_string"),n("span",{class:"token operator"},"="),s("_QUERY"),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),s("qres"),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token keyword"},"if"),s(" __name__ "),n("span",{class:"token operator"},"=="),s(),n("span",{class:"token string"},'"__main__"'),n("span",{class:"token punctuation"},":"),s(`
        client_init `),n("span",{class:"token operator"},"="),s(" SuiGQLClient"),n("span",{class:"token punctuation"},"("),s("config"),n("span",{class:"token operator"},"="),s("SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s("write_schema"),n("span",{class:"token operator"},"="),n("span",{class:"token boolean"},"False"),n("span",{class:"token punctuation"},")"),s(`
        main`),n("span",{class:"token punctuation"},"("),s("client_init"),n("span",{class:"token punctuation"},")"),s("```\n")])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1);function S(x,C){const i=c("CodeTabs");return u(),r("div",null,[m,l(i,{id:"84",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[k]),tab1:e(({value:a,isActive:t})=>[v]),_:1}),b,h,l(i,{id:"98",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[g]),tab1:e(({value:a,isActive:t})=>[y]),_:1}),_,f,l(i,{id:"112",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[q]),tab1:e(({value:a,isActive:t})=>[w]),_:1})])}const A=o(d,[["render",S],["__file","graphql.html.vue"]]);export{A as default};
