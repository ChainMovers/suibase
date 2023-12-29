import{_ as u}from"./plugin-vue_export-helper-c27b6911.js";import{r as o,o as p,c as r,a as n,b as s,d as i,w as a}from"./app-6bf8ea45.js";const d={},k=n("h2",{id:"facts",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#facts","aria-hidden":"true"},"#"),s(" Facts")],-1),m={class:"hint-container tip"},g=n("p",{class:"hint-container-title"},"Fact Sheet",-1),h=n("li",null,[s("Sui cli "),n("code",null,"sui client"),s(" will automatically generate a alias file (~/.sui/sui_config/sui.aliases) starting in version 1.16.0")],-1),_=n("li",null,"The alias file has a 1:1 mapping of alias names to the public key of the associated keypair",-1),f=n("li",null,"The alias name must start with a letter and can contain only letters, digits, hyphens (-), or underscores (_)",-1),v=n("li",null,[s("Command line caveats: "),n("ul",null,[n("li",null,"To rename an alias you will need to edit the alias file via editor"),n("li",null,"There is no known alias name length")])],-1),b=n("li",null,[s("pysui will check for alias file when using "),n("code",null,"default_config()"),s(", if not found it will generate one that complies with Sui 1.16.0 alias file format")],-1),y=n("li",null,[s("pysui's "),n("code",null,"SuiConfig"),s(" has methods to list, rename, use aliases for address and keypair lookups, and address or keypair lookup of aliases")],-1),w=n("li",null,"pysui enforces min and max aliases lengths to be between 3 and 64 characters. However; if alias name in alias file is modified manually pysui will continue to operate",-1),x=n("li",null,"An alias can be provided in the creation of new address/keypairs as well as recovering of same",-1),A={href:"https://pysui.readthedocs.io/en/latest/aliases.html",target:"_blank",rel:"noopener noreferrer"},C=n("h2",{id:"inspecting-aliases",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#inspecting-aliases","aria-hidden":"true"},"#"),s(" Inspecting aliases")],-1),S=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,`sui keytool list
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),T=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

`),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"alias_look"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Show the aliase, associated address and public key."""'),s(`
    cfg `),n("span",{class:"token operator"},"="),s(" SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token comment"},"# If running localnet w/suibase use this"),s(`
    `),n("span",{class:"token comment"},"# cfg = SuiConfig.sui_base_config()"),s(`
    `),n("span",{class:"token comment"},"# Loop through aliases and print"),s(`
    `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token keyword"},"for"),s(" alias "),n("span",{class:"token keyword"},"in"),s(" cfg"),n("span",{class:"token punctuation"},"."),s("aliases"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string-interpolation"},[n("span",{class:"token string"},'f"Alias:      '),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("alias"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"},'"')]),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string-interpolation"},[n("span",{class:"token string"},'f"Address:    '),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("cfg"),n("span",{class:"token punctuation"},"."),s("addr4al"),n("span",{class:"token punctuation"},"("),s("alias"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"},'"')]),n("span",{class:"token punctuation"},")"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string-interpolation"},[n("span",{class:"token string"},'f"PublicKey:  '),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("cfg"),n("span",{class:"token punctuation"},"."),s("pk4al"),n("span",{class:"token punctuation"},"("),s("alias"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"},'\\n"')]),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),I=n("h2",{id:"renaming-aliases",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#renaming-aliases","aria-hidden":"true"},"#"),s(" Renaming aliases")],-1),P=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,`sui keytool update-alias old_alias_name _new_alias_name_
`)]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),E=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(` SuiConfig

`),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"alias_rename"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Rename an alias."""'),s(`
    cfg `),n("span",{class:"token operator"},"="),s(" SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token comment"},"# If running localnet w/suibase use this"),s(`
    `),n("span",{class:"token comment"},"# cfg = SuiConfig.sui_base_config()"),s(`
    `),n("span",{class:"token comment"},"# Rename alias for the active_address"),s(`
    new_alias `),n("span",{class:"token operator"},"="),s(),n("span",{class:"token string"},'"Primary"'),s(`
    exiting_alias `),n("span",{class:"token operator"},"="),s(" cfg"),n("span",{class:"token punctuation"},"."),s("al4addr"),n("span",{class:"token punctuation"},"("),s("cfg"),n("span",{class:"token punctuation"},"."),s("active_address"),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string-interpolation"},[n("span",{class:"token string"},'f"Existing alias for active address '),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("cfg"),n("span",{class:"token punctuation"},"."),s("active_address"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"}," is "),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("exiting_alias"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"},'"')]),n("span",{class:"token punctuation"},")"),s(`
    cfg`),n("span",{class:"token punctuation"},"."),s("rename_alias"),n("span",{class:"token punctuation"},"("),s("old_alias"),n("span",{class:"token operator"},"="),s("exiting_alias"),n("span",{class:"token punctuation"},","),s(" new_alias"),n("span",{class:"token operator"},"="),s("new_alias"),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string-interpolation"},[n("span",{class:"token string"},`f"Address associated to new alias 'Primary' = `),n("span",{class:"token interpolation"},[n("span",{class:"token punctuation"},"{"),s("cfg"),n("span",{class:"token punctuation"},"."),s("addr4al"),n("span",{class:"token punctuation"},"("),s("new_alias"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},"}")]),n("span",{class:"token string"},'\\n"')]),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1),N=n("h2",{id:"using-aliases",tabindex:"-1"},[n("a",{class:"header-anchor",href:"#using-aliases","aria-hidden":"true"},"#"),s(" Using aliases")],-1),q=n("div",{class:"language-bash line-numbers-mode","data-ext":"sh"},[n("pre",{class:"language-bash"},[n("code",null,[s("Not applicable at this "),n("span",{class:"token function"},"time"),s(`
`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"})])],-1),B=n("div",{class:"language-python line-numbers-mode","data-ext":"py"},[n("pre",{class:"language-python"},[n("code",null,[n("span",{class:"token keyword"},"from"),s(" pysui "),n("span",{class:"token keyword"},"import"),s(" SyncClient"),n("span",{class:"token punctuation"},","),s(`SuiConfig

`),n("span",{class:"token keyword"},"def"),s(),n("span",{class:"token function"},"alias_use"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
    `),n("span",{class:"token triple-quoted-string string"},'"""Use alias to lookup address for transaciton."""'),s(`
    cfg `),n("span",{class:"token operator"},"="),s(" SuiConfig"),n("span",{class:"token punctuation"},"."),s("default_config"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token comment"},"# If running localnet w/suibase use this"),s(`
    `),n("span",{class:"token comment"},"# cfg = SuiConfig.sui_base_config()"),s(`
    client `),n("span",{class:"token operator"},"="),s(" SyncClient"),n("span",{class:"token punctuation"},"("),s("cfg"),n("span",{class:"token punctuation"},")"),s(`

    `),n("span",{class:"token comment"},"# Using alias for convenience"),s(`
    result `),n("span",{class:"token operator"},"="),s(" client"),n("span",{class:"token punctuation"},"."),s("execute"),n("span",{class:"token punctuation"},"("),s("GetAllCoins"),n("span",{class:"token punctuation"},"("),s("owner"),n("span",{class:"token operator"},"="),s("cfg"),n("span",{class:"token punctuation"},"."),s("addr4al"),n("span",{class:"token punctuation"},"("),n("span",{class:"token string"},'"Primary"'),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token keyword"},"if"),s(" result"),n("span",{class:"token punctuation"},"."),s("is_ok"),n("span",{class:"token punctuation"},"("),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),s("result"),n("span",{class:"token punctuation"},"."),s("result_data"),n("span",{class:"token punctuation"},"."),s("to_json"),n("span",{class:"token punctuation"},"("),s("indent"),n("span",{class:"token operator"},"="),n("span",{class:"token number"},"2"),n("span",{class:"token punctuation"},")"),n("span",{class:"token punctuation"},")"),s(`
    `),n("span",{class:"token keyword"},"else"),n("span",{class:"token punctuation"},":"),s(`
        `),n("span",{class:"token keyword"},"print"),n("span",{class:"token punctuation"},"("),s("result"),n("span",{class:"token punctuation"},"."),s("result_string"),n("span",{class:"token punctuation"},")"),s(`

`)])]),n("div",{class:"line-numbers","aria-hidden":"true"},[n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"}),n("div",{class:"line-number"})])],-1);function L(R,U){const c=o("ExternalLinkIcon"),l=o("CodeTabs");return p(),r("div",null,[k,n("div",m,[g,n("ul",null,[h,_,f,v,n("li",null,[s("PySui support of aliases: "),n("ul",null,[b,y,w,x,n("li",null,[s("pysui docs on "),n("a",A,[s("Aliases"),i(c)])])])])])]),C,i(l,{id:"74",data:[{id:"sui"},{id:"pysui"}]},{title0:a(({value:e,isActive:t})=>[s("sui")]),title1:a(({value:e,isActive:t})=>[s("pysui")]),tab0:a(({value:e,isActive:t})=>[S]),tab1:a(({value:e,isActive:t})=>[T]),_:1}),I,i(l,{id:"85",data:[{id:"sui"},{id:"pysui"}]},{title0:a(({value:e,isActive:t})=>[s("sui")]),title1:a(({value:e,isActive:t})=>[s("pysui")]),tab0:a(({value:e,isActive:t})=>[P]),tab1:a(({value:e,isActive:t})=>[E]),_:1}),N,i(l,{id:"96",data:[{id:"sui"},{id:"pysui"}]},{title0:a(({value:e,isActive:t})=>[s("sui")]),title1:a(({value:e,isActive:t})=>[s("pysui")]),tab0:a(({value:e,isActive:t})=>[q]),tab1:a(({value:e,isActive:t})=>[B]),_:1})])}const j=u(d,[["render",L],["__file","aliases.html.vue"]]);export{j as default};