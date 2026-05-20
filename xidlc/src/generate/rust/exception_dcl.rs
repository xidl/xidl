use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations,
    type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::ExceptDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let members = self
            .member
            .iter()
            .flat_map(|member| {
                let doc = doc_lines_from_annotations(&member.annotations);
                let rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
                let skip = hir::is_skipped(&member.annotations);
                member.ident.iter().map(move |decl| {
                    let name = crate::generate::rust::util::rust_ident(&declarator_name(decl));
                    let ty = type_with_decl(&member.ty, decl);
                    json!({
                        "ty": ty,
                        "name": name,
                        "serde_skip": skip,
                        "doc": doc,
                        "rust_attrs": rust_attrs,
                    })
                })
            })
            .collect::<Vec<_>>();
        let derive = rust_derive_info_with_extra(&self.annotations, &self.annotations);
        let ctx = renderer.enrich_ctx(
            renderer.with_ident(
                json!({
                    "members": members,
                    "derive": derive.all,
                    "enable_serde_attrs": derive.enable_serde_attrs(),
                    "rust_attrs": rust_passthrough_attrs_from_annotations(&self.annotations),
                }),
                &self.ident,
            ),
            &doc_lines_from_annotations(&self.annotations),
        );
        renderer.render_source_template("exception.rs.j2", &ctx)
    }
}
