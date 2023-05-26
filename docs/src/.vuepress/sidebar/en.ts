import { sidebar } from "vuepress-theme-hope";

export const enSidebar = sidebar({
  "/": [
    "",
    {
      text: "Getting Started",
      link: "intro.md",
      children: [
        {
          text: "What is Suibase?",
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
          text: "Devnet/Testnet/Mainnet",
          link: "how-to/devnet-testnet.md",
        },
      ],
    },
    {
      text: "Suibase Docs",
      link: "how-to/scripts.md",
      children: [
        {
          text: "Scripts",
          link: "how-to/scripts.md",
        },
        {
          text: "Suibase Helpers",
          link: "helpers.md",
        },
        {
          text: "Rust",
          collapsible: true,
          prefix: "rust/",
          children: [
            {
              text: "Helper API",
              link: "helper.md",
            },
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
              text: "Helper API",
              link: "helper.md",
            },
            {
              text: "Demos",
              link: "demos/README.md",
            },
          ],
        },
        {
          text: "Workdir Config",
          link: "how-to/configure-suibase-yaml.md",
        },
        {
          text: "Workdir Conventions",
          link: "references.md",
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
    {
      text: "Links",
      link: "links/",
      children: [
        {
          text: "External Resources",
          link: "links/",
        },
      ],
    },
  ],
});
