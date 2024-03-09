#![allow(dead_code)]
// Note:
//   This code will eventually move back into the DTP repos.
//   It is develop here for now...
//

// dtp-core crate is "internal" to DTP.
//
// See instead the dtp-sdk crate for the public API of DTP.
//
// dtp-core contains most of DTP implementation and complexity.
//
// dtp-sdk is a thin layer providing a simplified view to the user (facade pattern).
//
pub mod network;
pub mod types;
