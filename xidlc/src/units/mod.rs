//! Imported compilation units.
//!
//! An `import "pkg"` declaration names another compilation unit instead of
//! copying it. The compiler reads every unit it reaches, generates each one
//! independently, and hands the importing unit the prefixes its references use.
//! A directory import resolves to the `index.idl` inside it, a `@<version>`
//! suffix binds the default API version of that unit, and a cycle between units
//! is an error.

mod error;
mod graph;
mod interface;
mod path;
mod refs;
mod scan;
mod spec;

#[cfg(test)]
mod tests;

pub use error::ImportError;
pub use graph::UnitGraph;
pub use path::ImportPath;
pub use refs::UnitRef;
pub use spec::{ImportSpec, Unit};

/// Generator property that carries a unit's resolved imports into its target.
pub const IMPORT_PROPERTY: &str = "xidl_imports";
