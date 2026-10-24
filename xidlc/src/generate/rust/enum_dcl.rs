use crate::error::IdlcResult;
use crate::generate::rust::util::{
    rust_deprecated_attr, rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir::{self, Annotation};

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut has_default = false;
        let deprecated = self
            .member
            .iter()
            .map(|member| rust_deprecated_attr(&[&member.annotations]))
            .collect::<IdlcResult<Vec<_>>>()?;
        let any_variant_deprecated = deprecated.iter().any(|attr| attr.is_some());
        let mut members = self
            .member
            .iter()
            .zip(deprecated)
            .map(|(member, deprecated)| {
                let rust_name = crate::generate::rust::util::rust_ident(&member.ident);
                let wire_name =
                    hir::effective_wire_name(&member.ident, &member.annotations, &self.annotations);
                let rename = (wire_name != member.ident).then_some(wire_name);
                let doc = doc_lines_from_annotations(&member.annotations);
                let mut rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
                rust_attrs.extend(deprecated);
                let is_default = member
                    .annotations
                    .iter()
                    .any(|item| matches!(item, Annotation::DefaultLiteral));
                if is_default {
                    has_default = true;
                }
                json!({
                    "name": rust_name,
                    "serde_rename": rename,
                    "doc": doc,
                    "rust_attrs": rust_attrs,
                    "is_default": is_default
                })
            })
            .collect::<Vec<_>>();
        if !has_default {
            if let Some(v) = members.iter_mut().next() {
                v.as_object_mut()
                    .unwrap()
                    .insert("is_default".into(), true.into());
            }
        }
        let derive = rust_derive_info_with_extra(&self.annotations, &self.annotations);
        let mut rust_attrs = rust_passthrough_attrs_from_annotations(&self.annotations);
        // Derive expansions and `#[default]` name deprecated variants with
        // spans inside the enum, so the declaration silences the lint while
        // consumer code outside stays warned.
        if any_variant_deprecated {
            rust_attrs.push("allow(deprecated)".to_string());
        }
        rust_attrs.extend(rust_deprecated_attr(&[&self.annotations])?);
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "members": members,
                    "derive": derive.all,
                    "has_serde_serialize": derive.has_serde_serialize,
                    "has_serde_deserialize": derive.has_serde_deserialize,
                    "rust_attrs": rust_attrs,
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("enum.rs.j2", &ctx)
    }
}
