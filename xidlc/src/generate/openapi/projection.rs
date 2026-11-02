//! Public adapter for consumers of effective HTTP wire documentation.

use super::{OpenApiVersion, render_openapi_json, units::DocumentUnits};
use crate::error::IdlcResult;
use serde_json::Value;
use xidl_parser::{hir::ParserProperties, rest_hir::RestHirDocument};

/// Renders the existing HTTP schema/operation adapter without a file artifact.
pub struct OpenApiProjection;

impl OpenApiProjection {
    /// Reads documentation with the same annotation rules as XIDL generators.
    pub fn documentation(annotations: &[xidl_parser::hir::Annotation]) -> String {
        crate::generate::utils::doc::doc_lines_from_annotations(annotations).join("\n")
    }

    /// Projects effective wire shapes, descriptions, security and HTTP errors.
    pub fn render(document: &RestHirDocument, props: &ParserProperties) -> IdlcResult<Value> {
        render_openapi_json(
            &document.spec,
            document,
            &DocumentUnits::of(props),
            OpenApiVersion::Auto,
        )
    }
}
