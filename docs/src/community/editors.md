---
title: "How to become a Cookbook editor?"
order: 2
sidebarDepth: 0
---

Anyone can participate.

It is built from markdown files (.md) and served directly from [github](https://github.com/sui-base/sui-base/tree/main/docs/website). You submit changes by creating a branch with a pull request (just ask as needed).

**Editing the Cookbook**

You have to run [vuepress]( https://vuepress.vuejs.org/ ) and modify the markdown files with an editor (e.g. VSCode).

Requirements are Node.js and pnpm ( [More Info](https://theme-hope.vuejs.press/cookbook/tutorial/env.html) )

To start vuepress do:
```shell
$ cd ~/sui-base/docs
$ pnpm docs:dev
...
Open your browser at http://localhost:8080
```

The browser updates as you change files under ~/sui-base/docs.

Sui-base uses [https://theme-hope.vuejs.press/guide/](https://theme-hope.vuejs.press/guide/) for additional markdown features.
