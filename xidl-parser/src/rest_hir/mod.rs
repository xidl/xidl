mod attr;
mod exceptions;
pub mod mapping;
mod model;
mod project;
pub(crate) mod project_params;
mod route;
pub mod semantics;
#[cfg(test)]
mod tests;
mod types;
mod unions;
mod validate;

use serde::{Deserialize, Serialize};

pub use mapping::is_byte_sequence;
pub use model::ProjectOptions;
pub use model::*;
pub use project::{project, project_with_options};
pub use semantics::{UpgradeMode, WebSocketCodec, WebSocketConfig};

/// Selects the projected HIR shape produced from typed AST input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirProjectionKind {
    Rpc,
    Http,
    JsonRpc,
}

/// Represents either the standard DDS/RPC HIR or the projected REST HIR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectedHir {
    Rpc(crate::hir::Specification),
    Http(RestHirDocument),
    JsonRpc(crate::jsonrpc_hir::JsonRpcHirDocument),
}

pub(crate) type RestHirResult<T> = Result<T, String>;
