import * as path from "path";
import { fileURLToPath } from "url";
import { defineUserConfig } from "vuepress";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
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
    viteOptions: {
      // docs/src/python/demos and docs/src/rust/demo-app are symlinks that
      // resolve outside this package (see the "Fix contributors for Python
      // demos" commit). Vite/Rolldown resolve bare imports from a symlinked
      // page's real (out-of-tree) path, where "vue" isn't reachable via
      // normal node_modules walk-up. Alias it explicitly instead of
      // flipping resolve.preserveSymlinks, which breaks pnpm's own
      // virtual-store resolution for the bundler's internal packages.
      resolve: {
        alias: [
          {
            find: /^vue$/,
            replacement: path.resolve(__dirname, "../../node_modules/vue"),
          },
          {
            find: /^vue\//,
            replacement:
              path.resolve(__dirname, "../../node_modules/vue") + "/",
          },
        ],
      },
    },
    vuePluginOptions: {},
  }),

  locales: {
    "/": {
      lang: "en-US",
      title: "suibase.io",
      description:
        "Open-Source Sui Development Tools",
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
