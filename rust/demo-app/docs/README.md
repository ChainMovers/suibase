---
title: "Demo-App"
---

A good starting point for Rust+Move development on VSCode.

What to expect?

  * Rust+Move dApps that increment a Counter
  * The counter emit a Move event on every increment.
  * Rust app that subscribe and show all Sui Move events (do "cargo run events")
  * Rust app to send a transaction to increment the counter (do "cargo run count").
  * Uses the Suibase helper to get client address and the RPC URL.

To run this example, the Suibase installation is required.
To open the project, point VSCode on ~/suibase/rust/demo-app.

Online references: [Source Code](https://github.com/chainmovers/suibase/tree/main/rust/demo-app), [counter.move](https://github.com/chainmovers/suibase/tree/main/rust/demo-app/move/sources/counter.move)
