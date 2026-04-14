use std::fmt::Write;
use xidl_parser::hir;

use super::spec_types::{py_field_name, py_type, py_type_name};

pub(super) fn render_interface(out: &mut String, value: &hir::InterfaceDcl) {
    let hir::InterfaceDclInner::InterfaceDef(def) = &value.decl else {
        return;
    };
    writeln!(out, "class {}(abc.ABC):", py_type_name(&def.header.ident)).unwrap();
    let body = def.interface_body.as_ref().map(|body| &body.0);
    if body.map(|items| items.is_empty()).unwrap_or(true) {
        writeln!(out, "    pass").unwrap();
        writeln!(out).unwrap();
        return;
    }

    for export in body.unwrap() {
        if let hir::Export::OpDcl(op) = export {
            let ret = match &op.ty {
                hir::OpTypeSpec::Void => "None".to_string(),
                hir::OpTypeSpec::TypeSpec(value) => py_type(value),
            };
            let params = op
                .parameter
                .as_ref()
                .map(|params| {
                    params
                        .0
                        .iter()
                        .map(|param| {
                            format!(
                                "{}: {}",
                                py_field_name(&param.declarator.0),
                                py_type(&param.ty)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let suffix = if params.is_empty() {
                "self".to_string()
            } else {
                format!("self, {params}")
            };
            writeln!(out, "    @abc.abstractmethod").unwrap();
            writeln!(
                out,
                "    def {}({}) -> {}:",
                py_field_name(&op.ident),
                suffix,
                ret
            )
            .unwrap();
            writeln!(out, "        raise NotImplementedError").unwrap();
            writeln!(out).unwrap();
        }
    }
}
