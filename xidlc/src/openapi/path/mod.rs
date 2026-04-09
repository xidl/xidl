//! Implements [OpenAPI Path Object][paths] types.
//!
//! [paths]: https://spec.openapis.org/oas/latest.html#paths-object

mod operation;
mod parameter;
mod path_item;
mod paths;
#[cfg(test)]
mod tests;

pub use self::{
    operation::{Operation, OperationBuilder},
    parameter::{Parameter, ParameterBuilder, ParameterIn, ParameterStyle},
    path_item::{PathItem, PathItemBuilder},
    paths::{Paths, PathsBuilder},
};

use serde::{Deserialize, Serialize};

#[allow(missing_docs)]
#[doc(hidden)]
pub type PathsMap<K, V> = std::collections::BTreeMap<K, V>;

/// HTTP method of the operation.
///
/// List of supported HTTP methods <https://spec.openapis.org/oas/latest.html#path-item-object>
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    /// Type mapping for HTTP _GET_ request.
    Get,
    /// Type mapping for HTTP _POST_ request.
    Post,
    /// Type mapping for HTTP _PUT_ request.
    Put,
    /// Type mapping for HTTP _DELETE_ request.
    Delete,
    /// Type mapping for HTTP _OPTIONS_ request.
    Options,
    /// Type mapping for HTTP _HEAD_ request.
    Head,
    /// Type mapping for HTTP _PATCH_ request.
    Patch,
    /// Type mapping for HTTP _TRACE_ request.
    Trace,
}
