mod attr;
mod model;
mod project;
mod project_params;
mod route;
pub mod semantics;
#[cfg(test)]
mod tests;
mod validate;

use serde::{Deserialize, Serialize};

pub use model::*;
pub use project::project;

/// Selects the projected HIR shape produced from typed AST input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirProjectionKind {
    Rpc,
    Http,
    JsonRpc,
}

/// Represents either the standard DDS/RPC HIR or the projected HTTP HIR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectedHir {
    Rpc(crate::hir::Specification),
    Http(HttpHirDocument),
    JsonRpc(crate::jsonrpc_hir::JsonRpcHirDocument),
}

pub(crate) type HttpHirResult<T> = Result<T, String>;
