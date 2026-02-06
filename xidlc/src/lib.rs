pub mod cli;
pub mod driver;
pub mod error;
pub mod fmt;
pub mod generate;
pub mod highlight;
pub mod jsonrpc;
pub mod macros;
pub(crate) mod transport;

use std::collections::HashMap;
use std::path::Path;

pub fn generate_from_source(
    lang: &str,
    idl: &str,
    props: HashMap<String, serde_json::Value>,
) -> Result<Vec<driver::File>, error::IdlcError> {
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

#[cfg(target_os = "emscripten")]
pub mod wasm_api;
