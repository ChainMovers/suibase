import{_ as u}from"./plugin-vue_export-helper-c27b6911.js";import{r as c,o as r,c as p,a as n,b as s,d as i,w as e,e as d}from"./app-6bf8ea45.js";const m={},k=n("h2",{id:"facts",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#facts","aria-hidden":"true"},"#"),s(" Facts")],-1),b={class:"hint-container tip"},v=n("p",{class:"hint-container-title"},"Fact Sheet",-1),h=n("li",null,[s("Sui GraphQL RPC is currently in "),n("strong",null,[n("em",null,"beta")])],-1),_=n("li",null,[s("Sui GraphQL RPC beta operates on a snapshot of data with timestamps: "),n("ul",null,[n("li",null,'testnet: "2023-12-16T19:07:30.993Z"'),n("li",null,'mainnet: "2023-11-21T22:03:27.667Z"'),n("li",null,"devnet not currently supported")])],-1),g=n("li",null,[s("Sui GraphQL RPC will eventually "),n("em",null,"replace"),s(" the JSON RPC")],-1),y={href:"https://docs.sui.io/references/sui-api/beta-graph-ql#using-sui-graphql-rpc",target:"_blank",rel:"noopener noreferrer"},f=d("<li>Release 0.50.0 includes an &#39;experimental&#39; implementation, subject to change</li><li>Provides Synchronous and asynchronous GraphQL clients</li><li>Only &#39;read&#39; queries are supported at the time of this writing</li><li>Introduces <code>QueryNodes</code> that are the equivalent to pysui <code>Builders</code></li><li>Parity of QueryNodes to Builders is ongoing</li><li>Exposes ability for developers to write their own GraphQL queries</li><li>SuiConfiguration must point to either Sui&#39;s <code>testnet</code> or <code>mainnet</code> RPC URLs</li>",7),q={href:"https://pysui.readthedocs.io/en/latest/graphql.html",target:"_blank",rel:"noopener noreferrer"},w=n("h2",{id:"generating-graphql-schema",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#generating-graphql-schema","aria-hidden":"true"},"#"),s(" Generating GraphQL schema")],-1),S=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,`NA
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),x=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Dump Sui GraphQL Schema."""'),s(`
        `),n("span",{class:"token comment"},"# Initialize synchronous client (must be mainnet or testnet)"),s(`
        client_init `),n("span",{class:"token operator"},"="),s(" SuiGQLClient"),n("span",{class:"token punctuation"},"("),s("config"),n("span",{class:"token operator"},"="),s("SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s("write_schema"),n("span",{class:"token operator"},"="),n("span",{class:"token boolean"},"True"),n("span",{class:"token punctuation"},")"),s(`

        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string"},'"Schema dumped to: `latest_schemaVERSION.graqhql`"'),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token keyword"},"if"),s(" __name__ "),n("span",{class:"token operator"},"=="),s(),n("span",{class:"token string"},'"__main__"'),n("span",{class:"token punctuation"},":"),s(`
        main`),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),Q=n("h2",{id:"query-example-1",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#query-example-1","aria-hidden":"true"},"#"),s(" Query example 1")],-1),C=n("p",null,[s("For pysui there are 3 comon ways to create a query. This demonstrates "),n("strong",null,[n("em",null,"using QueryNodes (predefined queries as part of pysui SDK)")])],-1),G=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[s("NA at this "),n("span",{class:"token function"},"time"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),L=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token comment"},"#"),s(`
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
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),A=n("h2",{id:"query-example-2",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#query-example-2","aria-hidden":"true"},"#"),s(" Query example 2")],-1),N=n("p",null,[s("For pysui there are 3 comon ways to create a query. This demonstrates "),n("strong",null,[n("em",null,"using a query string")])],-1),R=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[n("span",{class:"token comment"},"# basic query"),s(`
`),n("span",{class:"token function"},"curl"),s(),n("span",{class:"token parameter variable"},"-X"),s(" POST https://graphql-beta.mainnet.sui.io "),n("span",{class:"token punctuation"},"\\"),s(`
     `),n("span",{class:"token parameter variable"},"--header"),s(),n("span",{class:"token string"},'"Content-Type: application/json"'),s(),n("span",{class:"token punctuation"},"\\"),s(`
     `),n("span",{class:"token parameter variable"},"--data"),s(),n("span",{class:"token string"},`'{
          "query": "query { epoch { referenceGasPrice } }"
     }'`),s(`
`),n("span",{class:"token comment"},"# query with variables"),s(`
`),n("span",{class:"token function"},"curl"),s(),n("span",{class:"token parameter variable"},"-X"),s(" POST https://graphql-beta.mainnet.sui.io "),n("span",{class:"token punctuation"},"\\"),s(`
     `),n("span",{class:"token parameter variable"},"--header"),s(),n("span",{class:"token string"},'"Content-Type: application/json"'),s(),n("span",{class:"token punctuation"},"\\"),s(`
     `),n("span",{class:"token parameter variable"},"--data"),s(),n("span",{class:"token string"},`'{
          "query": "query ($epochID: Int!) { epoch(id: $epochID) { referenceGasPrice } }", "variables": { "epochID": 123 }
     }'`),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),I=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token comment"},"#"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Query using a query string."""'),s(`

    `),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),s("client"),n("span",{class:"token punctuation"},":"),s(" SuiGQLClient"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Execute a static string query."""'),s(`
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
        main`),n("span",{class:"token punctuation"},"("),s("client_init"),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),T=n("h2",{id:"query-example-3",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#query-example-3","aria-hidden":"true"},"#"),s(" Query example 3")],-1),D={href:"https://github.com/graphql-python/gql",target:"_blank",rel:"noopener noreferrer"},P=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,`NA
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),E=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[s("    "),n("span",{class:"token comment"},"#"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Query using gql DocumentNode."""'),s(`
    `),n("span",{class:"token keyword"},"from"),s(" gql "),n("span",{class:"token keyword"},"import"),s(` gql
    `),n("span",{class:"token keyword"},"from"),s(" pysui"),n("span",{class:"token punctuation"},"."),s("sui"),n("span",{class:"token punctuation"},"."),s("sui_pgql"),n("span",{class:"token punctuation"},"."),s("clients "),n("span",{class:"token keyword"},"import"),s(` SuiGQLClient
    `),n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

    `),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"main"),n("span",{class:"token punctuation"},"("),s("client"),n("span",{class:"token punctuation"},":"),s(" SuiGQLClient"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token triple-quoted-string string"},'"""Execute a compiled string into DocumentNode."""'),s(`
        _QUERY `),n("span",{class:"token operator"},"="),s(),n("span",{class:"token comment"},"# Same query string as used Query example 2"),s(`
        qres `),n("span",{class:"token operator"},"="),s(" client"),n("span",{class:"token punctuation"},"."),s("execute_query"),n("span",{class:"token punctuation"},"("),s("with_document_node"),n("span",{class:"token operator"},"="),s("gql"),n("span",{class:"token punctuation"},"("),s("_QUERY"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),s("qres"),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token keyword"},"if"),s(" __name__ "),n("span",{class:"token operator"},"=="),s(),n("span",{class:"token string"},'"__main__"'),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token comment"},"# Initialize synchronous client (must be mainnet or testnet)"),s(`
        client_init `),n("span",{class:"token operator"},"="),s(" SuiGQLClient"),n("span",{class:"token punctuation"},"("),s("config"),n("span",{class:"token operator"},"="),s("SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},","),s("write_schema"),n("span",{class:"token operator"},"="),n("span",{class:"token boolean"},"False"),n("span",{class:"token punctuation"},")"),s(`
        main`),n("span",{class:"token punctuation"},"("),s("client_init"),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1);function F(U,V){const o=c("ExternalLinkIcon"),l=c("CodeTabs");return r(),p("div",null,[k,n("div",b,[v,n("ul",null,[h,_,g,n("li",null,[s("Sui support and constraints defined "),n("a",y,[s("Here"),i(o)])]),n("li",null,[s("PySui support for Sui GraphQL RPC: "),n("ul",null,[f,n("li",null,[s("pysui GraphQL documentation is in the "),n("a",q,[s("Docs"),i(o)])])])])])]),w,i(l,{id:"94",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[S]),tab1:e(({value:a,isActive:t})=>[x]),_:1}),Q,C,i(l,{id:"108",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[G]),tab1:e(({value:a,isActive:t})=>[L]),_:1}),A,N,i(l,{id:"122",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[R]),tab1:e(({value:a,isActive:t})=>[I]),_:1}),T,n("p",null,[s("For pysui there are 3 comon ways to create a query. This demonstrates "),n("strong",null,[n("em",null,[s("using "),n("a",D,[s("gql"),i(o)]),s(" the underlying GraphQL library")])]),s(" to generate a DocumentNode")]),i(l,{id:"136",data:[{id:"sui"},{id:"pysui"}]},{title0:e(({value:a,isActive:t})=>[s("sui")]),title1:e(({value:a,isActive:t})=>[s("pysui")]),tab0:e(({value:a,isActive:t})=>[P]),tab1:e(({value:a,isActive:t})=>[E]),_:1})])}const B=u(m,[["render",F],["__file","graphql.html.vue"]]);export{B as default};
