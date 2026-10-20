//! Project a rendered transport module and its fields.

use super::super::scope::TypeScope;
use super::model::{TransportTypeDef, TransportVariantContext, TypeRegistry};
use super::names::transport_ident;
use super::{
    TransportDirection, TransportFieldContext, TransportItemContext, TransportModuleContext,
    decode_expr, encode_expr, map_type_inner,
};
use crate::error::IdlcResult;
use crate::generate::rust::util::{
    array_type, declarator_dims, declarator_name, is_skipped_from_annotations, rust_ident,
};
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
        match def {
            TransportTypeDef::Struct(def) => items.push(render_struct(
                name,
                def,
                direction,
                module_name,
                registry,
                module_path,
            )?),
            TransportTypeDef::Enum(def) => items.push(render_enum(name, def, module_path)),
            TransportTypeDef::Typedef(_) | TransportTypeDef::Public => {}
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
    let scope = TypeScope {
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
            let raw = declarator_name(decl);
            let name = rust_ident(&raw);
            let ty = member_ty(member, decl, direction, module_name, scope)?;
            let wire = hir::effective_wire_name(&raw, &member.annotations, &def.annotations);
            let serde_rename = (wire != name).then_some(wire);
            let (enc, dec) = if member.is_optional() {
                let e = encode_expr("value", &member.ty, scope)?;
                let d = decode_expr("value", &member.ty, scope)?;
                let enc = if e == "value" {
                    format!("value.{name}")
                } else {
                    format!("value.{name}.map(|value| {e})")
                };
                let dec = if d == "value" {
                    format!("value.{name}")
                } else {
                    format!("value.{name}.map(|value| {d})")
                };
                (enc, dec)
            } else {
                (
                    encode_expr(&format!("value.{name}"), &member.ty, scope)?,
                    decode_expr(&format!("value.{name}"), &member.ty, scope)?,
                )
            };
            fields.push(TransportFieldContext {
                flatten: crate::generate::utils::has_annotation(&member.annotations, "flatten"),
                name: name.clone(),
                ty,
                serde_rename,
                optional: member.is_optional(),
                encode_expr: enc,
                decode_expr: dec,
            });
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

fn member_ty(
    member: &hir::Member,
    decl: &hir::Declarator,
    direction: TransportDirection,
    module_name: &str,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    let mut base = map_type_inner(&member.ty, direction, module_name, scope, None)?;
    if member.is_optional() {
        base = format!("Option<{base}>");
    }
    let dims = declarator_dims(decl);
    Ok(if dims.is_empty() {
        base
    } else {
        array_type(&base, &dims)
    })
}
