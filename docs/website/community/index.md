---
hide:
  - toc
---
# Sui-Base is Community Driven

## Where is the Sui developer community?
For Sui-Base (+any 3rd party dev tool welcome to join):

  * [Sui-Base Discord :octicons-link-external-16:](https://discord.com/invite/Erb6SwsVbH)

For Sui specific discussions:

  * [Sui Official Discord :octicons-link-external-16:](https://discord.gg/sui)

  * [Sui Official Forum :octicons-link-external-16:](https://forums.sui.io/)


## How to be a writer for the Cookbook?

Anyone can participate.

The website is built from markdown files (.md) and served directly from [github :octicons-link-external-16:](https://github.com/sui-base/sui-base/tree/main/docs/website). You submit changes with a pull request.

**Running/Editing the website**

You can run the whole website locally with just pip and git (See [here :octicons-link-external-16:](https://squidfunk.github.io/mkdocs-material/getting-started/) for more ways to install).

??? abstract "Installation Steps"

    ``` console
    $ cd ~
    $ git clone https://github.com/sui-base/sui-base.git
    $ cd sui-base/docs
    $ python3 -m venv env; source env/bin/activate
    $ pip install mkdocs-material
    $ pip install mkdocs-git-revision-date-localized-plugin
    $ pip install mkdocs-minify-plugin
    $ mkdocs serve
    ...
    Open your browser at http://127.0.0.1:8000/
    ```

The server updates your browser automatically as you update the files under ~/sui-base/docs.

Edit [mkdocs.yml :octicons-link-external-16:]( https://github.com/sui-base/sui-base/blob/main/docs/mkdocs.yml ) to add new subject to the left navigation bar.

Check [mkdocs-material :octicons-link-external-16:]( https://squidfunk.github.io/mkdocs-material/reference/ ) for great markdown tricks.
