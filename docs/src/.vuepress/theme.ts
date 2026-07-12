import { hopeTheme } from "vuepress-theme-hope";
import { enNavbar /*, zhNavbar*/ } from "./navbar/index.js";
import { enSidebar /*, zhSidebar*/ } from "./sidebar/index.js";

// The Sui Cookbook was split out into its own site:
// https://chainmovers.github.io/sui-cookbook/
// Keep the old suibase.io/cookbook/* URLs working by redirecting each one to
// its 1:1 counterpart on the new site (generates a static redirect stub per
// path at build time).
const COOKBOOK_SITE = "https://chainmovers.github.io/sui-cookbook";
const cookbookPaths = [
  "", // /cookbook/            -> new site home
  "code/", // section indexes
  "guides/",
  "code/Tokens.html",
  "code/aliases.html",
  "code/graphql.html",
  "code/keypairs.html",
  "code/multisig.html",
  "code/networks.html",
  "code/objects.html",
  "code/transactions.html",
  "guides/build.html",
  "guides/fast_transactions.html",
  "guides/fullnode.html",
  "guides/progtxns.html",
  "guides/sui-intro.html",
];
const cookbookRedirects: Record<string, string> = Object.fromEntries(
  cookbookPaths.map((p) => [`/cookbook/${p}`, `${COOKBOOK_SITE}/${p}`]),
);

export default hopeTheme({
  hostname: "https://suibase.io",

  author: {
    name: "suibase.io",
    url: "https://suibase.io",
  },

  logo: "/logo.png",

  repo: "chainmovers/suibase",

  contributors: false,
  editLink: false,
  pageInfo: false,
  breadcrumb: false,
  toc: false,

  docsDir: "docs/src/",

  hotReload: true,

  markdown: {
    align: true,
    attrs: true,
    chartjs: true,
    echarts: false,
    flowchart: true,
    gfm: true,
    include: true,
    mark: true,
    mermaid: true,
    stylize: [
      {
        matcher: "Recommended",
        replacer: ({ tag }) => {
          if (tag === "em")
            return {
              tag: "Badge",
              attrs: { type: "tip" },
              content: "Recommended",
            };
        },
      },
    ],
    sub: true,
    sup: true,
    vPre: true,
    vuePlayground: false,

    figure: false,
    imgLazyload: true,
    imgMark: true,
    imgSize: true,

    math: true,

    alert: true,
    hint: true,

    tabs: true,
    codeTabs: true,
  },

  locales: {
    "/": {
      // navbar
      navbar: enNavbar,

      // sidebar
      sidebar: enSidebar,

      footer:
        '<a href="https://github.com/chainmovers/suibase">Suibase on Github</a>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<a href="https://discord.com/invite/Erb6SwsVbH">Suibase on Discord</a>',

      copyright: "Apache 2.0 Open-Source License",

      displayFooter: true,

      metaLocales: {
        editLink: "Edit this page on GitHub",
      },
    },

    /**
     * Chinese locale config
     *
    "/zh/": {
      // navbar
      navbar: zhNavbar,

      // sidebar
      sidebar: zhSidebar,

      footer: "默认页脚",

      displayFooter: true,

      // page meta
      metaLocales: {
        editLink: "在 GitHub 上编辑此页",
      },
    },*/
  },

  plugins: {
    /*
    comment: {
      // @ts-expect-error: You should generate and use your own comment service
      provider: "Waline",
    },*/
    git: true,

    icon: {
      assets: "iconify",
    },

    // Redirect the retired /cookbook/* pages (and the cookbook-editor guide)
    // to the standalone Sui Cookbook.
    redirect: {
      config: {
        ...cookbookRedirects,
        "/community/editors.html": `${COOKBOOK_SITE}/editors.html`,
      },
    },

    docsearch: {
      // your options
      // appId, apiKey and indexName are required
      appId: "VN5D5IVTPC", // gitleaks:allow
      apiKey: "7c6732e9f43a129ee2396d1c459db319", // gitleaks:allow
      indexName: "sui-base",
    },

    // uncomment these if you want a pwa
    // pwa: {
    //   favicon: "/favicon.ico",
    //   cacheHTML: true,
    //   cachePic: true,
    //   appendBase: true,
    //   apple: {
    //     icon: "/assets/icon/apple-icon-152.png",
    //     statusBarColor: "black",
    //   },
    //   msTile: {
    //     image: "/assets/icon/ms-icon-144.png",
    //     color: "#ffffff",
    //   },
    //   manifest: {
    //     icons: [
    //       {
    //         src: "/assets/icon/chrome-mask-512.png",
    //         sizes: "512x512",
    //         purpose: "maskable",
    //         type: "image/png",
    //       },
    //       {
    //         src: "/assets/icon/chrome-mask-192.png",
    //         sizes: "192x192",
    //         purpose: "maskable",
    //         type: "image/png",
    //       },
    //       {
    //         src: "/assets/icon/chrome-512.png",
    //         sizes: "512x512",
    //         type: "image/png",
    //       },
    //       {
    //         src: "/assets/icon/chrome-192.png",
    //         sizes: "192x192",
    //         type: "image/png",
    //       },
    //     ],
    //     shortcuts: [
    //       {
    //         name: "Demo",
    //         short_name: "Demo",
    //         url: "/demo/",
    //         icons: [
    //           {
    //             src: "/assets/icon/guide-maskable.png",
    //             sizes: "192x192",
    //             purpose: "maskable",
    //             type: "image/png",
    //           },
    //         ],
    //       },
    //     ],
    //   },
    // },
  },
});
