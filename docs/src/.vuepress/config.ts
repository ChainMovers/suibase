import { defineUserConfig } from "vuepress";
import theme from "./theme.js";
import { searchProPlugin } from "vuepress-plugin-search-pro";

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
    searchProPlugin({
      // index all contents
      indexContent: true,
    }),
  ],

  // Enable it with pwa
  // shouldPrefetch: false,
});
