// Common test utilities for suibase-daemon integration tests.
//
// Each integration test compiles `mod common` independently and only uses
// a subset of the helpers, so the rest look "dead" to that test binary.
// Suppress dead-code warnings module-wide — this is the canonical Rust
// pattern for shared test utilities with `-D warnings` enabled at the
// workspace level (see rust/suibase/.cargo/config.toml).
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod mock_test_utils;

pub use mock_test_utils::*;