pub mod cli;
pub mod diagnostic;
pub mod driver;
pub mod error;
pub mod fmt;
pub mod generate;
pub mod jsonrpc;
pub mod macros;

use std::collections::HashMap;
use std::path::Path;

use crate::error::IdlcResult;

pub fn generate_from_source(
    lang: &str,
    idl: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<driver::File>> {
    let mut generator = driver::Generator::new(lang.into());
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(generator.generate_from_idl(idl, Path::new("input.idl"), props))
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|err| error::IdlcError::fmt(err.to_string()))?;
        rt.block_on(generator.generate_from_idl(idl, Path::new("input.idl"), props))
    }
}
