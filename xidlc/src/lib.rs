#[cfg(feature = "cli")]
pub mod cli;
pub mod diagnostic;
pub mod driver;
pub mod error;
pub mod fmt;
pub mod generate;
pub mod import;
pub mod jsonrpc;
pub mod macros;
pub mod openapi;

use std::collections::HashMap;
use std::path::Path as FsPath;

use crate::error::IdlcResult;

pub trait PartialSchema {
    fn schema() -> openapi::RefOr<openapi::schema::Schema>;
}

pub trait ToSchema: PartialSchema {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(std::any::type_name::<Self>())
    }

    fn schemas(_schemas: &mut Vec<(String, openapi::RefOr<openapi::schema::Schema>)>) {}
}

pub trait Path {
    fn methods() -> Vec<openapi::path::HttpMethod>;
    fn path() -> String;
    fn operation() -> openapi::path::Operation;
}

pub trait IntoResponses {
    fn responses() -> std::collections::BTreeMap<String, openapi::RefOr<openapi::response::Response>>;
}

pub trait ToResponse<'r> {
    fn response() -> (&'r str, openapi::RefOr<openapi::response::Response>);
}

pub fn generate_from_source(
    lang: &str,
    idl: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<driver::File>> {
    let mut generator = driver::Generator::new(lang.into());
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(generator.generate_from_idl(idl, FsPath::new("input.idl"), props))
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|err| error::IdlcError::fmt(err.to_string()))?;
        rt.block_on(generator.generate_from_idl(idl, FsPath::new("input.idl"), props))
    }
}
