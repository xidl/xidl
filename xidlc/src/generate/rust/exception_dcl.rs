use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, rust_deprecated_attr, rust_derive_info_with_extra,
    rust_passthrough_attrs_from_annotations, type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;
use xidl_parser::rest_hir::semantics::has_annotation;

impl RustRender for hir::ExceptDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        // Like structs: the whole exception silences the lint when its
        // members reach deprecated declarations, so field type paths and
        // derive expansions stay quiet while consumers stay warned.
        let allow_deprecated = self
            .member
            .iter()
            .any(|member| member.references_deprecated)
            || rust_deprecated_attr(&[&self.annotations])?.is_some();
        let deprecated = self
            .member
            .iter()
            .map(|member| rust_deprecated_attr(&[&member.annotations]))
            .collect::<IdlcResult<Vec<_>>>()?;
        let members = self
            .member
            .iter()
            .zip(deprecated)
            .flat_map(|(member, deprecated)| {
                let doc = doc_lines_from_annotations(&member.annotations);
                let mut rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
                rust_attrs.extend(deprecated);
                // `@header`/`@cookie` members travel outside the body, so the
                // serialized error body excludes them; generated client/server
                // code moves the values through real headers instead.
                let skip = hir::is_skipped(&member.annotations)
                    || has_annotation(&member.annotations, "header")
                    || has_annotation(&member.annotations, "cookie");
                member.ident.iter().map(move |decl| {
                    let raw_name = declarator_name(decl);
                    let name = crate::generate::rust::util::rust_ident(&raw_name);
                    let mut ty = type_with_decl(&member.ty, decl);
                    let optional = member.is_optional();
                    if optional {
                        ty = format!("Option<{ty}>");
                    }
                    let wire_name =
                        hir::effective_wire_name(&raw_name, &member.annotations, &self.annotations);
                    let rename = (wire_name != raw_name).then_some(wire_name);
                    json!({
                        "ty": ty,
                        "name": name,
                        "serde_skip": skip,
                        "optional": optional,
                        "serde_rename": rename,
                        "doc": doc,
                        "rust_attrs": rust_attrs,
                    })
                })
            })
            .collect::<Vec<_>>();
        let derive = rust_derive_info_with_extra(&self.annotations, &self.annotations);
        let mut rust_attrs = rust_passthrough_attrs_from_annotations(&self.annotations);
        if allow_deprecated {
            rust_attrs.push("allow(deprecated)".to_string());
        }
        rust_attrs.extend(rust_deprecated_attr(&[&self.annotations])?);
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "members": members,
                    "derive": derive.all,
                    "enable_serde_attrs": derive.enable_serde_attrs(),
                    "rust_attrs": rust_attrs,
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("exception.rs.j2", &ctx)
    }
}
