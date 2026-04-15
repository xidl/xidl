use crate::error::IdlcResult;
use crate::generate::rust::util::{
    rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations,
    serde_rename_from_annotations,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir::{self, Annotation};

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut has_default = false;
        let mut members = self
            .member
            .iter()
            .map(|member| {
                let rust_name = crate::generate::rust::util::rust_ident(&member.ident);
                let rename = serde_rename_from_annotations(&member.annotations);
                let doc = doc_lines_from_annotations(&member.annotations);
                let rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
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
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "members": members,
                    "has_serde_serialize": derive.has_serde_serialize,
                    "has_serde_deserialize": derive.has_serde_deserialize,
                    "rust_attrs": rust_passthrough_attrs_from_annotations(&self.annotations),
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("enum.rs.j2", &ctx)
    }
}
