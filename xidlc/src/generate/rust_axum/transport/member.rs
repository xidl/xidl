//! Keep transport member layout and owned conversions aligned with public models.

use super::super::{render::RustAxumRenderer, scope::TypeScope};
use super::{TransportDirection, TransportFieldContext, convert::OwnedConversion, map_type_inner};
use crate::error::IdlcResult;
use crate::generate::rust::util::{array_type, declarator_dims, declarator_name, rust_ident};
use xidl_parser::hir;

/// Projects a declared member using the same layout layers as its public model.
pub(super) struct MemberProjection<'a> {
    member: &'a hir::Member,
    owner: &'a hir::StructDcl,
    scope: TypeScope<'a>,
    renderer: &'a RustAxumRenderer,
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
        let conversion = OwnedConversion::new(self.scope, self.renderer);
        let converted = if self.member.is_optional() {
            let inner = self.boxed_conversion(&conversion, "value", dimensions.len())?;
            conversion.map(&source, &inner)?
        } else {
            self.boxed_conversion(&conversion, &source, dimensions.len())?
        };
        Ok(TransportFieldContext {
            flatten: crate::generate::utils::has_annotation(&self.member.annotations, "flatten"),
            serde_skip: ["header", "cookie"]
                .iter()
                .any(|name| crate::generate::utils::has_annotation(&self.member.annotations, name)),
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

    fn boxed_conversion(
        &self,
        conversion: &OwnedConversion<'_>,
        source: &str,
        dimensions: usize,
    ) -> IdlcResult<String> {
        if self.member.recursive {
            let converted =
                conversion.array(&format!("(*{source})"), &self.member.ty, dimensions)?;
            conversion.boxed(&converted)
        } else {
            conversion.array(source, &self.member.ty, dimensions)
        }
    }
}
