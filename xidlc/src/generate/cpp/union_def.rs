use crate::error::IdlcResult;
use crate::generate::cpp::util::{
    apply_declarator, collect_inline_defs, cpp_scoped_name, cpp_switch_type, declarator_name,
    render_const, type_with_decl,
};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use std::collections::BTreeSet;
use xidl_parser::hir;

impl CppRender for hir::UnionForwardDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let ctx = json!({ "kind": "class", "ident": &self.ident });
        let rendered = renderer.render_template("forward.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}

impl CppRender for hir::UnionDef {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();

        for case in &self.case {
            if let hir::ElementSpecTy::ConstrTypeDcl(constr) = &case.element.ty {
                out.extend(collect_inline_defs(constr, renderer)?);
            }
        }

        let mut fields = Vec::new();
        let mut seen = BTreeSet::new();
        for case in &self.case {
            let name = declarator_name(&case.element.value);
            if seen.insert(name.clone()) {
                fields.push((name, &case.element.ty, &case.element.value));
            }
        }

        let members = fields
            .into_iter()
            .map(|(name, ty, decl)| {
                let ty = match ty {
                    hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, decl),
                    hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                        let base = cpp_scoped_name(&constr_type_name(constr));
                        apply_declarator(&base, decl)
                    }
                };
                json!({ "name": name, "ty": ty })
            })
            .collect::<Vec<_>>();

        let cases = self
            .case
            .iter()
            .map(|case| {
                let labels = case
                    .label
                    .iter()
                    .map(|label| match label {
                        hir::CaseLabel::Value(expr) => render_const(expr),
                        hir::CaseLabel::Default => "default".to_string(),
                    })
                    .collect::<Vec<_>>();
                json!({
                    "labels": labels,
                    "member": declarator_name(&case.element.value),
                })
            })
            .collect::<Vec<_>>();

        let ctx = json!({
            "ident": &self.ident,
            "switch_ty": cpp_switch_type(&self.switch_type_spec),
            "members": members,
            "cases": cases,
        });
        let rendered = renderer.render_template("union.h.j2", &ctx)?;
        out.header.push(rendered);
        Ok(out)
    }
}

fn constr_type_name(constr: &hir::ConstrTypeDcl) -> hir::ScopedName {
    let name = match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    };
    hir::ScopedName {
        name: vec![name],
        is_root: false,
    }
}
