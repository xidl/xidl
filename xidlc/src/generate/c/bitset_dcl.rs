use crate::error::IdlcResult;
use crate::generate::c::util::{bitfield_type, c_scoped_name_hir};
use crate::generate::c::xcdr::{kind_json, type_kind_from_c};
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitsetDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let fields = self
            .field
            .iter()
            .map(|field| {
                let ty = field
                    .ty
                    .as_ref()
                    .map(bitfield_type)
                    .unwrap_or_else(|| "uint32_t".to_string());
                let pos = crate::generate::render_const_expr(
                    &field.pos.0,
                    &crate::generate::c::util::c_scoped_name,
                    &crate::generate::c::util::c_literal,
                );
                let mask = format!("(1u << ({}))", pos);
                json!({
                    "ident": &field.ident,
                    "ty": ty,
                    "pos": pos,
                    "mask": mask,
                })
            })
            .collect::<Vec<_>>();

        let xcdr_fields = self
            .field
            .iter()
            .map(|field| {
                let ty = field
                    .ty
                    .as_ref()
                    .map(bitfield_type)
                    .unwrap_or_else(|| "uint32_t".to_string());
                let kind = type_kind_from_c(&ty);
                json!({
                    "expr": format!("self->{}", field.ident),
                    "dims": Vec::<String>::new(),
                    "kind": kind_json(&kind),
                })
            })
            .collect::<Vec<_>>();

        let all_mask = if fields.is_empty() {
            "0".to_string()
        } else {
            fields
                .iter()
                .map(|field| field["mask"].as_str().unwrap_or("0"))
                .collect::<Vec<_>>()
                .join(" | ")
        };

        let ctx = json!({
            "ident": &self.ident,
            "parent": self.parent.as_ref().map(c_scoped_name_hir),
            "fields": fields,
            "all_mask": all_mask,
            "xcdr_fields": xcdr_fields,
        });

        let header = renderer.render_template("bitset.h.j2", &ctx)?;
        let source = renderer.render_template("bitset.c.j2", &ctx)?;
        let xcdr_header = renderer.render_template("bitset_xcdr.h.j2", &ctx)?;
        let xcdr_source = renderer.render_template("bitset_xcdr.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source)
            .push_xcdr_header(xcdr_header)
            .push_xcdr_source(xcdr_source))
    }
}
