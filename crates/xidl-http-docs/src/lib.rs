//! Compilation-scoped HTTP API documentation composed from compiler-owned contracts.

mod changes;
mod config;
mod contract;
mod documentation;
mod error;
mod idl;
mod links;
mod markdown;
mod notes;
mod page;
mod plugin;
mod render;
mod schema;
mod site;
mod summary;
mod wire;

pub use config::DocumentationConfig;
pub use error::DocumentationError;
pub use plugin::DocumentationPlugin;
