//! Map model types using the selected target runtime's collection primitives.

use super::{RustRenderer, util};
use crate::error::{IdlcError, IdlcResult};
use serde::Serialize;
use xidl_parser::hir;

/// An IDL collection's corresponding Rust container family.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CollectionKind {
    Sequence,
    Map,
}

impl RustRenderer {
    /// Selects the runtime providing bounded model containers for this target.
    #[cfg(any(feature = "gen-rust-axum", feature = "gen-rust-axum-adapters"))]
    pub(crate) fn with_runtime(mut self, runtime: &'static str) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Renders one type, including bounded containers at every nesting level.
    pub(crate) fn type_name(&self, ty: &hir::TypeSpec) -> IdlcResult<String> {
        match ty {
            hir::TypeSpec::SequenceType(value) => self.collection_type(
                CollectionKind::Sequence,
                &[self.type_name(&value.ty)?],
                value.len.as_ref(),
            ),
            hir::TypeSpec::MapType(value) => self.collection_type(
                CollectionKind::Map,
                &[self.type_name(&value.key)?, self.type_name(&value.value)?],
                value.len.as_ref(),
            ),
            hir::TypeSpec::TemplateType(value) => Ok(format!(
                "{}<{}>",
                util::rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(|ty| self.type_name(ty))
                    .collect::<IdlcResult<Vec<_>>>()?
                    .join(", "),
            )),
            hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
                self.render_template(
                    "json_value.rs.j2",
                    &serde_json::json!({ "runtime": self.runtime }),
                )
            }
            _ => Ok(util::rust_type(ty)),
        }
    }

    /// Wraps a base type in its exact fixed-array dimensions.
    pub(crate) fn declared_type(
        &self,
        ty: &hir::TypeSpec,
        decl: &hir::Declarator,
    ) -> IdlcResult<String> {
        Ok(util::array_type(
            &self.type_name(ty)?,
            &util::declarator_dims(decl),
        ))
    }

    /// Renders a collection using already mapped elements and normalized bounds.
    pub(crate) fn collection_type(
        &self,
        kind: CollectionKind,
        elements: &[String],
        bound: Option<&hir::PositiveIntConst>,
    ) -> IdlcResult<String> {
        let maximum = if self.runtime.is_some() {
            bound
                .map(|bound| {
                    hir::const_expr_to_i64(&bound.0)
                        .filter(|value| *value > 0)
                        .ok_or_else(|| {
                            IdlcError::rpc(
                                "Rust collection bound must be normalized before rendering",
                            )
                        })
                })
                .transpose()?
        } else {
            None
        };
        self.render_template(
            "collection.rs.j2",
            &serde_json::json!({
                "kind": kind, "elements": elements, "maximum": maximum, "runtime": self.runtime,
            }),
        )
    }
}
