use crate::diagnostic::DiagnosticBag;
use crate::hir::{ParserProperties, Specification};
use crate::jsonrpc_hir;
use crate::rest_hir;
use crate::versioning::VersionedProjection;
use crate::versioning::map::VersionMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Selects the projected HIR shape produced from typed AST input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirProjectionKind {
    /// Language-neutral RPC HIR (`hir::Specification`).
    Rpc,
    /// Shared HTTP semantic model (`rest_hir::RestHirDocument`).
    Http,
    /// Shared JSON-RPC semantic model (`jsonrpc_hir::JsonRpcHirDocument`).
    JsonRpc,
}

/// A source specification or its HTTP or JSON-RPC projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectedHir {
    /// Language-neutral RPC specification.
    Rpc(Specification),
    /// Projected HTTP REST document.
    Http(rest_hir::RestHirDocument),
    /// Projected JSON-RPC document.
    JsonRpc(jsonrpc_hir::JsonRpcHirDocument),
}

/// Configuration for a single-pass [`super::Pipeline::compile_source`] invocation.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Parser and generator properties.
    pub properties: ParserProperties,
    /// Logical source file path used for include resolution and contract naming.
    pub path: PathBuf,
    /// Whether to build a [`VersionMap`] alongside projected outputs.
    pub include_version_map: bool,
    /// Whether HTTP operations should be included in the built [`VersionMap`].
    pub include_http_operations: bool,
}

impl CompileOptions {
    /// Creates compilation options for `path` with `properties`.
    pub fn new(path: impl Into<PathBuf>, properties: ParserProperties) -> Self {
        Self {
            properties,
            path: path.into(),
            include_version_map: false,
            include_http_operations: false,
        }
    }

    /// Enables [`VersionMap`] generation from the same lowered source HIR.
    pub fn with_version_map(mut self, include_http_operations: bool) -> Self {
        self.include_version_map = true;
        self.include_http_operations = include_http_operations;
        self
    }
}

/// Output of a single-pass compilation through [`super::Pipeline`].
#[derive(Debug)]
pub struct CompiledUnit {
    /// Projected HIR outputs (one per selected version or one merged output).
    pub projections: Vec<VersionedProjection>,
    /// Adjacent-version metadata when the unit declares a `@version` timeline.
    pub version_map: Option<VersionMap>,
    /// Diagnostics accumulated across syntax, AST, and HIR passes.
    pub diagnostics: DiagnosticBag,
}
