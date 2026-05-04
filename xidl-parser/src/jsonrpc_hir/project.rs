use std::collections::HashSet;

use crate::error::ParserResult;
use crate::hir;

use super::attr::project_attr;
use super::semantics::{param_is_input, param_is_output, stream_kind};
use super::{
    JsonRpcField, JsonRpcFieldSource, JsonRpcHirDocument, JsonRpcInterface, JsonRpcMethod,
    JsonRpcMethodKind, JsonRpcMethodSource, JsonRpcResponseKind,
};

#[cfg(test)]
mod tests;

pub fn project(spec: &hir::Specification) -> ParserResult<JsonRpcHirDocument> {
    let mut interfaces = Vec::new();
    collect_interfaces(&spec.0, &mut Vec::new(), &mut interfaces)?;
    Ok(JsonRpcHirDocument {
        spec: spec.clone(),
        interfaces,
    })
}

fn collect_interfaces(
    defs: &[hir::Definition],
    module_path: &mut Vec<String>,
    interfaces: &mut Vec<JsonRpcInterface>,
) -> ParserResult<()> {
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                module_path.push(module.ident.clone());
                collect_interfaces(&module.definition, module_path, interfaces)?;
                module_path.pop();
            }
            hir::Definition::InterfaceDcl(interface) => {
                interfaces.push(project_interface(interface, module_path)?);
            }
            _ => {}
        }
    }
    Ok(())
}

fn project_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
) -> ParserResult<JsonRpcInterface> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(JsonRpcInterface {
            ident: match &interface.decl {
                hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.ident.clone(),
                hir::InterfaceDclInner::InterfaceDef(def) => def.header.ident.clone(),
            },
            module_path: module_path.to_vec(),
            annotations: interface.annotations.clone(),
            methods: Vec::new(),
            watch_methods: Vec::new(),
        });
    };
    let user_ops = collect_user_ops(def);
    let mut methods = Vec::new();
    let mut watch_methods = Vec::new();

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(project_op(op, &def.header.ident, module_path)?)
                }
                hir::Export::AttrDcl(attr) => {
                    let (attr_methods, attr_watches) =
                        project_attr(attr, &def.header.ident, module_path, &user_ops)?;
                    methods.extend(attr_methods);
                    watch_methods.extend(attr_watches);
                }
                _ => {}
            }
        }
    }

    Ok(JsonRpcInterface {
        ident: def.header.ident.clone(),
        module_path: module_path.to_vec(),
        annotations: interface.annotations.clone(),
        methods,
        watch_methods,
    })
}

fn collect_user_ops(def: &hir::InterfaceDef) -> HashSet<&str> {
    let mut out = HashSet::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            if let hir::Export::OpDcl(op) = export {
                out.insert(op.ident.as_str());
            }
        }
    }
    out
}

fn project_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
) -> ParserResult<JsonRpcMethod> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    Ok(JsonRpcMethod {
        source: JsonRpcMethodSource::Operation,
        kind: stream_kind(&op.annotations)?.unwrap_or(JsonRpcMethodKind::Unary),
        name: op.ident.clone(),
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        annotations: op.annotations.clone(),
        request_fields: request_fields(params),
        response_fields: response_fields(op, params),
        response_kind: response_kind(op, params),
        stream_item: None,
    })
}

fn request_fields(params: &[hir::ParamDcl]) -> Vec<JsonRpcField> {
    params
        .iter()
        .filter(|param| param_is_input(param.attr.as_ref()))
        .map(param_field)
        .collect()
}

fn response_fields(op: &hir::OpDcl, params: &[hir::ParamDcl]) -> Vec<JsonRpcField> {
    let mut out = Vec::new();
    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        out.push(return_field(ty));
    }
    out.extend(
        params
            .iter()
            .filter(|param| param_is_output(param.attr.as_ref()))
            .map(param_field),
    );
    out
}

fn response_kind(op: &hir::OpDcl, params: &[hir::ParamDcl]) -> JsonRpcResponseKind {
    let has_return = matches!(op.ty, hir::OpTypeSpec::TypeSpec(_));
    match (has_return, response_fields(op, params).len()) {
        (_, 0) => JsonRpcResponseKind::Empty,
        (true, 1) => JsonRpcResponseKind::SingleReturn,
        (false, 1) => JsonRpcResponseKind::SingleOutput,
        _ => JsonRpcResponseKind::MultiOutput,
    }
}

fn param_field(param: &hir::ParamDcl) -> JsonRpcField {
    let name = param.declarator.0.clone();
    JsonRpcField {
        name: name.clone(),
        wire_name: name,
        ty: param.ty.clone(),
        source: JsonRpcFieldSource::Param,
    }
}

fn return_field(ty: &hir::TypeSpec) -> JsonRpcField {
    JsonRpcField {
        name: "return".to_string(),
        wire_name: "return".to_string(),
        ty: ty.clone(),
        source: JsonRpcFieldSource::Return,
    }
}

fn rpc_method_name(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
