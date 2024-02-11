//import * as path from "path";
import { defineUserConfig } from "vuepress";
import theme from "./theme.js";
//import { searchProPlugin } from "vuepress-plugin-search-pro";
//import { docsearchPlugin } from "@vuepress/plugin-docsearch";
//import { registerComponentsPlugin } from "@vuepress/plugin-register-components";
//import { redirectPlugin } from "vuepress-plugin-redirect";
import { googleAnalyticsPlugin } from "@vuepress/plugin-google-analytics";
import { viteBundler } from "@vuepress/bundler-vite";

export default defineUserConfig({
  base: "/",

  bundler: viteBundler({
    viteOptions: {},
    vuePluginOptions: {},
  }),

  locales: {
    "/": {
      lang: "en-US",
      title: "suibase.io",
      description:
        "Sui Network Open-Source Development Tools and Community Cookbook",
    },
  },

  theme,

  plugins: [
    /*
    registerComponentsPlugin({
      componentsDir: path.resolve(__dirname, "./components"),
    }),*/
    googleAnalyticsPlugin({
      id: "G-JVE9L5ZDYZ",
    }),
  ],

  // Enable it with pwa
  // shouldPrefetch: false,
});
