use crate::error::IdlcResult;
use crate::generate::c::util::field_for_member;
use crate::generate::c::xcdr::{declarator_info, kind_json, type_kind};
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::StructDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let members = self
            .member
            .iter()
            .flat_map(|member| {
                member
                    .ident
                    .iter()
                    .map(|decl| field_for_member(member, decl))
            })
            .collect::<Vec<_>>();

        let mut xcdr_fields = Vec::new();
        for member in &self.member {
            let kind = type_kind(&member.ty);
            for decl in &member.ident {
                let (name, dims) = declarator_info(decl);
                xcdr_fields.push(json!({
                    "expr": format!("self->{}", name),
                    "dims": dims,
                    "kind": kind_json(&kind),
                }));
            }
        }

        let ctx = json!({
            "ident": &self.ident,
            "members": members,
            "xcdr_fields": xcdr_fields,
        });
        let header = renderer.render_template("struct.h.j2", &ctx)?;
        let source = renderer.render_template("struct.c.j2", &ctx)?;
        let xcdr_header = renderer.render_template("struct_xcdr.h.j2", &ctx)?;
        let xcdr_source = renderer.render_template("struct_xcdr.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source)
            .push_xcdr_header(xcdr_header)
            .push_xcdr_source(xcdr_source))
    }
}
