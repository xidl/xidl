//! Codegen plugin contract: shared types plus a small synchronous RPC.
//!
//! Built-in generators are called in-process. External plugins (`xidl-<lang>`)
//! speak the NDJSON stdio protocol defined in [`ipc`].

mod compilation;
mod ipc;

pub use compilation::{
    COMPILATION_FORMAT, CompilationInput, CompilationUnit, UnitImport, VersionInput,
};
pub use ipc::*;
