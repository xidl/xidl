#[cfg(feature = "cli")]
pub mod cli;
pub mod diagnostic;
pub mod driver;
pub mod error;
pub mod fmt;
pub mod generate;
pub mod import;
pub mod jsonrpc;
pub mod library;
pub mod macros;
pub mod openapi;
pub mod units;
#[cfg(feature = "cli")]
mod version_diff;
#[cfg(feature = "cli")]
mod version_lock;

use std::collections::HashMap;
use std::path::Path as FsPath;

use crate::error::IdlcResult;

/// Generates target artifacts directly from an in-memory IDL source string.
pub fn generate_from_source(
    lang: &str,
    idl: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<driver::File>> {
    let mut generator = driver::Generator::new(lang.into());
    generator.generate_from_idl(idl, FsPath::new("input.idl"), props)
}
