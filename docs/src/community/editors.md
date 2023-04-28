---
title: "How to become a Cookbook editor?"
order: 2
headerDepth: 0
contributors: true
editLink: true
---

Anyone with a Github account can participate.

The cookbook is built from markdown files (.md) and served directly from [github](https://github.com/sui-base/suibase/tree/main/docs/website).

## Editing on Github (very easy/quick changes)

Open the editor with the "Edit this pages on Github" link at the bottom.

When ready to propose your changes just select "Create a **new branch**" and give it a name:<br>
<img :src="$withBase('/assets/propose-change.png')" alt="Propose Changes"><br>

Your proposed changes will be merged after review.

## Editing the Cookbook on your machine (more serious editor)
If you prefer to preview exactly how your change will be displayed, then you need to run [vuepress]( https://vuepress.vuejs.org/ ) on your own and modify the markdown files with an editor (e.g. VSCode).

Requirements are Node.js and pnpm ( [More Info](https://theme-hope.vuejs.press/cookbook/tutorial/env.html) )

To start vuepress do:
```shell
$ cd ~/suibase/docs
$ pnpm docs:dev
...
Open your browser at http://localhost:8080
```

The browser updates as you change files under ~/suibase/docs.

Suibase uses [https://theme-hope.vuejs.press/guide/](https://theme-hope.vuejs.press/guide/) for additional markdown features.

Submit your changes as a pull request, just ask as needed (not as hard as it seems once you go through it once).