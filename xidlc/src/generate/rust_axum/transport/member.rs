//! Keep transport member layout and owned conversions aligned with public models.

use super::super::{render::RustAxumRenderer, scope::TypeScope};
use super::{TransportDirection, TransportFieldContext, convert::convert_expr, map_type_inner};
use crate::error::IdlcResult;
use crate::generate::rust::util::{array_type, declarator_dims, declarator_name, rust_ident};
use serde::Serialize;
use xidl_parser::hir;

/// Projects a declared member using the same layout layers as its public model.
pub(super) struct MemberProjection<'a> {
    member: &'a hir::Member,
    owner: &'a hir::StructDcl,
    scope: TypeScope<'a>,
    renderer: &'a RustAxumRenderer,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Conversion<'a> {
    Map { source: &'a str, converted: &'a str },
    Box { converted: &'a str },
}

impl<'a> MemberProjection<'a> {
    /// Shares the declaration scope and template renderer for one member.
    pub(super) fn new(
        member: &'a hir::Member,
        owner: &'a hir::StructDcl,
        scope: TypeScope<'a>,
        renderer: &'a RustAxumRenderer,
    ) -> Self {
        Self {
            member,
            owner,
            scope,
            renderer,
        }
    }

    /// Builds the wire field and both owned conversions from its declared shape.
    pub(super) fn render(
        &self,
        decl: &hir::Declarator,
        direction: TransportDirection,
        module_name: &str,
    ) -> IdlcResult<TransportFieldContext> {
        let raw = declarator_name(decl);
        let name = rust_ident(&raw);
        let wire =
            hir::effective_wire_name(&raw, &self.member.annotations, &self.owner.annotations);
        let dimensions = declarator_dims(decl);
        let ty = self.wire_type(&dimensions, direction, module_name)?;
        let source = format!("value.{name}");
        let converted = if self.member.is_optional() {
            let inner = self.boxed_conversion("value", dimensions.len())?;
            self.map(&source, &inner)?
        } else {
            self.boxed_conversion(&source, dimensions.len())?
        };
        Ok(TransportFieldContext {
            flatten: crate::generate::utils::has_annotation(&self.member.annotations, "flatten"),
            name: name.clone(),
            ty,
            serde_rename: (wire != name).then_some(wire),
            optional: self.member.is_optional(),
            encode_expr: converted.clone(),
            decode_expr: converted,
        })
    }

    fn wire_type(
        &self,
        dimensions: &[String],
        direction: TransportDirection,
        module_name: &str,
    ) -> IdlcResult<String> {
        let mut ty = map_type_inner(&self.member.ty, direction, module_name, self.scope, None)?;
        if !dimensions.is_empty() {
            ty = array_type(&ty, dimensions);
        }
        if self.member.recursive {
            ty = format!("Box<{ty}>");
        }
        if self.member.is_optional() {
            ty = format!("Option<{ty}>");
        }
        Ok(ty)
    }

    fn boxed_conversion(&self, source: &str, dimensions: usize) -> IdlcResult<String> {
        if self.member.recursive {
            let converted = self.array_conversion(&format!("(*{source})"), dimensions)?;
            self.renderer.render_template(
                "interface/transport_expression.rs.j2",
                &Conversion::Box {
                    converted: &converted,
                },
            )
        } else {
            self.array_conversion(source, dimensions)
        }
    }

    fn array_conversion(&self, source: &str, dimensions: usize) -> IdlcResult<String> {
        if dimensions == 0 {
            return convert_expr(source, &self.member.ty, self.scope);
        }
        let converted = self.array_conversion("value", dimensions - 1)?;
        self.map(source, &converted)
    }

    fn map(&self, source: &str, converted: &str) -> IdlcResult<String> {
        if converted == "value" {
            return Ok(source.to_owned());
        }
        self.renderer.render_template(
            "interface/transport_expression.rs.j2",
            &Conversion::Map { source, converted },
        )
    }
}
