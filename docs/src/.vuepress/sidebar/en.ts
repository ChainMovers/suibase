import { sidebar } from "vuepress-theme-hope";

export const enSidebar = sidebar({
  "/": [
    "",
    {
      text: "Getting Started",
      link: "intro.md",
      children: [
        {
          text: "What is Sui-Base?",
          link: "intro.md",
        },
        {
          text: "Installation",
          link: "how-to/install.md",
        },
        {
          text: "Localnet",
          link: "how-to/localnet.md",
        },
        {
          text: "Devnet/Testnet",
          link: "how-to/devnet-testnet.md",
        },
      ],
    },
    {
      text: "Sui-Base Docs",
      link: "how-to/scripts.md",
      children: [
        {
          text: "Scripts",
          link: "how-to/scripts.md",
        },
        {
          text: "Workdir Conventions",
          link: "references.md",
        },
        {
          text: "Sui-Base Helpers",
          link: "helpers.md",
        },
        {
          text: "Workdir Config",
          link: "how-to/configure-sui-base-yaml.md",
        },
        {
          text: "Rust",
          collapsible: true,
          prefix: "rust/",
          children: [
            {
              text: "Demo-App",
              link: "demo-app/README.md",
            },
          ],
        },
        {
          text: "Python",
          collapsible: true,
          prefix: "python/",
          children: [
            {
              text: "Demos",
              link: "demos/README.md",
            },
          ],
        },
      ],
    },
    {
      text: "Sui Cookbook",
      prefix: "cookbook/",
      link: "cookbook/README.md",
      children: [
        {
          text: "Introduction",
          link: "README.md",
        },
        {
          text: "Guides",
          collapsible: true,
          prefix: "guides/",
          children: "structure",
        },
        {
          text: "Code Snippets",
          collapsible: true,
          prefix: "code/",
          children: "structure",
        },
        {
          text: "SDK List",
          link: "sdk-list.md",
        },
      ],
    },
    {
      text: "Community",
      link: "community/",
      children: [
        {
          text: "Forums / Contacts",
          link: "community/",
        },
        {
          text: "Become an Editor",
          link: "community/editors.md",
        },
      ],
    },
  ],
});
