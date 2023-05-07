---
title: Suibase Helpers
---

Suibase Helpers are APIs providing what is needed to initialize Sui Network SDKs.

That includes basic params such as the active client address and valid RPC URL.

Some other params needed in a typical "edit/publish/test" dev cycle are *your modules* package and shared_object IDs that you last published.

These IDs are generated at publication time by the Sui client and written in a JSON file. Suibase automatically preserve this file in the workdir, and makes the IDs easily readable by any Rust/Python apps through the Helper API.

#### Example 1: What is the active client address of localnet?
Demo TBD

#### Example 2: What is my last published package ID?
Demo TBD

#### Example 3: Which URL should be used right now for localnet?
Suibase monitor RPC health of multiple fullnode and return the best URL to use.
Demo TBD
