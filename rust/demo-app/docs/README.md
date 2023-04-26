---
title: "Demo-App"
---

A good starting point for Rust+Move development on VSCode.

What to expect?

  * A simple Rust+Move dApps that increment a Counter on your localnet.
  * The counter emit a Move event on every increment.
  * Rust app that subscribe and show all Sui Move events (do "cargo run events")
  * Rust app to send a transaction to increment the counter (do "cargo run count").
  * Use of suibase scripts and helper to accelerate and automate Sui development.

To run this example, Suibase installation is required.
To open the project, point VSCode on ~/rust/demo-app.

Online references: [Source Code](https://github.com/suibase/suibase/tree/main/rust/demo-app), [counter.move](https://github.com/suibase/suibase/tree/main/rust/demo-app/move/sources/counter.move)
