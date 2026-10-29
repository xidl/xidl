//! Project a rendered transport module and its fields.

use super::super::scope::TypeScope;
use super::member::MemberProjection;
use super::model::{TransportTypeDef, TransportVariantContext, TypeRegistry};
use super::names::transport_ident;
use super::{
    TransportDirection, TransportFieldContext, TransportItemContext, TransportModuleContext,
    map_type_inner,
};
use crate::error::IdlcResult;
use crate::generate::rust::util::{is_skipped_from_annotations, rust_ident};
use std::collections::BTreeSet;
use xidl_parser::hir;

pub(super) fn render_module(
    names: &BTreeSet<String>,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<TransportModuleContext> {
    let mut items = Vec::new();
    for name in names {
        let Some(def) = registry.get(name) else {
            continue;
        };
        if let Some(record) = def.record() {
            items.push(render_struct(
                name,
                &record,
                direction,
                module_name,
                registry,
                module_path,
            )?);
        } else if let TransportTypeDef::Enum(def) = def {
            items.push(render_enum(name, def, module_path));
        }
    }
    Ok(TransportModuleContext {
        name: module_name.to_string(),
        items,
    })
}

fn render_struct(
    canonical: &str,
    def: &hir::StructDcl,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<TransportItemContext> {
    let mut output = module_path.to_vec();
    output.push(module_name.to_string());
    let mut declaration = canonical
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    declaration.pop();
    let renderer = super::super::render::RustAxumRenderer::new()?;
    let scope = TypeScope {
        types: &renderer.models,
        registry,
        declaration: &declaration,
        output: &output,
    };
    let mut fields = Vec::new();
    if let Some(parent) = def.parent.first() {
        let ty = map_type_inner(
            &hir::TypeSpec::ScopedName(parent.clone()),
            direction,
            module_name,
            scope,
            None,
        )?;
        fields.push(TransportFieldContext {
            flatten: true,
            serde_skip: false,
            name: "base".to_string(),
            ty,
            serde_rename: None,
            optional: false,
            encode_expr: "value.base.into()".to_string(),
            decode_expr: "value.base.into()".to_string(),
        });
    }
    for member in &def.member {
        if is_skipped_from_annotations(&member.annotations) {
            continue;
        }
        for decl in &member.ident {
            fields.push(MemberProjection::new(member, def, scope, &renderer).render(
                decl,
                direction,
                module_name,
            )?);
        }
    }
    Ok(TransportItemContext {
        kind: "struct".to_string(),
        transport_ident: transport_ident(canonical),
        public_path: TypeScope::relative_path(scope.output, canonical),
        has_skipped_members: def
            .member
            .iter()
            .any(|member| is_skipped_from_annotations(&member.annotations)),
        fields,
        variants: Vec::new(),
    })
}

fn render_enum(
    canonical: &str,
    def: &hir::EnumDcl,
    module_path: &[String],
) -> TransportItemContext {
    TransportItemContext {
        kind: "enum".to_string(),
        transport_ident: transport_ident(canonical),
        public_path: format!(
            "super::{}",
            TypeScope::relative_path(module_path, canonical)
        ),
        has_skipped_members: false,
        fields: Vec::new(),
        variants: def
            .member
            .iter()
            .map(|item| {
                let ident = rust_ident(&item.ident);
                let wire =
                    hir::effective_wire_name(&item.ident, &item.annotations, &def.annotations);
                let serde_rename = (wire != ident).then_some(wire);
                TransportVariantContext {
                    ident,
                    serde_rename,
                }
            })
            .collect(),
    }
}
