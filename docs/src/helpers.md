---
title: Suibase Helpers
---

Suibase Helpers are APIs providing what is needed to initialize a Sui Network SDKs to target any network.

That includes basic params such as the active client address and valid RPC URL.

Some other params needed in a typical "edit/publish/test" dev cycle are *your modules* package ID and object IDs of the shared objects that you last published (presumably on localnet, devnet or testnet).

These IDs are generated at publication time by the Sui client. Suibase makes it convenient by preserving the JSON file (See [Filesystem Path Convention]( ./references.md#filesystem-path-convention) for more details).

This JSON file is then conveniently readable by your apps through a Suibase Helper API.

#### Example 1: What is the active client address of localnet?
TBD

#### Example 2: What is my last published package ID?
TBD



