#![forbid(unsafe_code)]

// Control-side gateway relay client implementation follows in this branch.
// The manifest is committed first so GitHub CI can generate the exact locked
// rustls dependency graph without local builds.

pub const CONTROL_GATEWAY_CLIENT_PROTOCOL_VERSION: u32 = 1;
