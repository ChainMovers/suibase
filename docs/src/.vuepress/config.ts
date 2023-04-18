import { defineUserConfig } from "vuepress";
import theme from "./theme.js";
//import { searchProPlugin } from "vuepress-plugin-search-pro";
import { docsearchPlugin } from "@vuepress/plugin-docsearch";

export default defineUserConfig({
  base: "/",

  locales: {
    "/": {
      lang: "en-US",
      title: "sui-base.io",
      description:
        "Sui Network Open-Source Development Tools and Community Cookbook",
    },
  },

  theme,

  plugins: [
    /*
    searchProPlugin({
      // index all contents
      indexContent: true,
    }),*/
    docsearchPlugin({
      // your options
      // appId, apiKey and indexName are required
      appId: "VN5D5IVTPC",
      apiKey: "7c6732e9f43a129ee2396d1c459db319",
      indexName: "sui-base",
    }),
  ],

  // Enable it with pwa
  // shouldPrefetch: false,
});
