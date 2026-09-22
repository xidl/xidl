//! Codegen plugin contract: shared types plus a small synchronous RPC.
//!
//! Built-in generators are called in-process. External plugins (`xidl-<lang>`)
//! speak the NDJSON stdio protocol defined in [`ipc`].

mod ipc;

pub use ipc::*;
