use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let members = self
            .member
            .iter()
            .map(|member| crate::generate::rust::util::rust_ident(&member.ident))
            .collect::<Vec<_>>();
        let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
            &self.annotations,
            &self.annotations,
        );
        let has_serde_serialize = derive.iter().any(|d| d == "::serde::Serialize");
        let has_serde_deserialize = derive.iter().any(|d| d == "::serde::Deserialize");
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "members": members,
            "has_serde_serialize": has_serde_serialize,
            "has_serde_deserialize": has_serde_deserialize,
            "typeobject_path": renderer.typeobject_path(),
        });
        let rendered = renderer.render_template("enum.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
