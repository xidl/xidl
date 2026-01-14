use crate::error::IdlcResult;
use crate::generate::c::util::{
    c_element_type_name, c_switch_type, collect_inline_defs, declarator_info,
};
use crate::generate::c::xcdr::{
    declarator_info as xcdr_declarator_info, element_kind, kind_json, switch_kind,
};
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use crate::generate::render_const_expr;
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::UnionDef {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let mut out = CRenderOutput::default();

        for case in &self.case {
            if let hir::ElementSpecTy::ConstrTypeDcl(constr) = &case.element.ty {
                out.extend(collect_inline_defs(constr, renderer)?);
            }
        }

        let cases = self
            .case
            .iter()
            .map(|case| {
                let labels = case
                    .label
                    .iter()
                    .map(|label| {
                        crate::generate::render_const_expr(
                            label,
                            &crate::generate::c::util::c_scoped_name,
                            &crate::generate::c::util::c_literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let element_type = c_element_type_name(&case.element.ty);
                let (name, dims) = declarator_info(&case.element.value);
                json!({
                    "labels": labels,
                    "ty": element_type,
                    "name": name,
                    "dims": dims,
                })
            })
            .collect::<Vec<_>>();

        let switch_kind = switch_kind(&self.switch_type_spec);
        let xcdr_switch_field = json!({
            "expr": "self->_d",
            "dims": Vec::<String>::new(),
            "kind": kind_json(&switch_kind),
        });
        let xcdr_cases = self
            .case
            .iter()
            .map(|case| {
                let labels = case
                    .label
                    .iter()
                    .map(|label| {
                        render_const_expr(
                            label,
                            &crate::generate::c::util::c_scoped_name,
                            &crate::generate::c::util::c_literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let kind = element_kind(&case.element.ty);
                let (name, dims) = xcdr_declarator_info(&case.element.value);
                json!({
                    "labels": labels,
                    "field": {
                        "expr": format!("self->_u.{}", name),
                        "dims": dims,
                        "kind": kind_json(&kind),
                    }
                })
            })
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": &self.ident,
            "switch_ty": c_switch_type(&self.switch_type_spec),
            "cases": cases,
            "xcdr_switch_field": xcdr_switch_field,
            "xcdr_cases": xcdr_cases,
        });

        let header = renderer.render_template("union.h.j2", &ctx)?;
        let source = renderer.render_template("union.c.j2", &ctx)?;
        let xcdr_header = renderer.render_template("union_xcdr.h.j2", &ctx)?;
        let xcdr_source = renderer.render_template("union_xcdr.c.j2", &ctx)?;
        out.header.push(header);
        out.source.push(source);
        out.xcdr_header.push(xcdr_header);
        out.xcdr_source.push(xcdr_source);
        Ok(out)
    }
}
