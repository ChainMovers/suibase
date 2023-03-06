# demo-app

A good starting point for Rust+Move development on VSCode.

What to expect?

  * A simple Rust+Move dApps that increment a Counter on your localnet.
  * The counter emit a Move event on every increment.
  * Rust app that subscribe and show all Sui Move events (do "cargo run events") 
  * Rust app to send a transaction to increment the counter (do "cargo run count").
  * Use of sui-base scripts and helper to accelerate and automate Sui development.

To run this example, Sui-base installation is required.
To open the project, point VSCode on ~/sui-base/rust/demo-app.

Online references: [Source Code :octicons-mark-github-16:](https://github.com/sui-base/sui-base/tree/main/rust/demo-app), [counter.move :octicons-mark-github-16:](https://github.com/sui-base/sui-base/tree/main/rust/demo-app/move/sources/counter.move)
