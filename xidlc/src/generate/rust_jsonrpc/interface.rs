use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use serde::Serialize;
use xidl_parser::hir;

#[derive(Serialize)]
struct ParamField {
    name: String,
    ty: String,
}

#[derive(Serialize)]
struct MethodContext {
    name: String,
    params: Vec<String>,
    params_fields: Vec<ParamField>,
    params_struct: String,
    ret: String,
    rpc_name: String,
    args: Vec<String>,
}

pub fn render_interface_with_path(
    interface: &hir::InterfaceDcl,
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    match &interface.decl {
        hir::InterfaceDclInner::InterfaceForwardDcl(_) => Ok(JsonRpcRenderOutput::default()),
        hir::InterfaceDclInner::InterfaceDef(def) => {
            render_interface_def(def, renderer, module_path)
        }
    }
}

fn render_interface_def(
    def: &hir::InterfaceDef,
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    let mut out = JsonRpcRenderOutput::default();
    let mut methods = Vec::new();

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(render_op(op, &def.header.ident, module_path));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(render_attr(attr, &def.header.ident, module_path));
                }
                _ => {}
            }
        }
    }

    let ctx = serde_json::json!({
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodContext {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => jsonrpc_type(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    let mut param_fields = Vec::new();
    let mut args = Vec::new();
    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let name = rust_ident(&param.declarator.0);
        param_list.push(format!("{name}: {ty}"));
        param_fields.push(ParamField {
            name: name.clone(),
            ty,
        });
        args.push(name);
    }

    let method_name = rust_ident(&op.ident);
    MethodContext {
        name: method_name.clone(),
        params: param_list,
        params_fields: param_fields,
        params_struct: params_struct_name(interface_name, &method_name),
        ret,
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        args,
    }
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
) -> Vec<MethodContext> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|names| {
                let ret = attr_return_type(&spec.ty);
                MethodContext {
                    name: names.rust.clone(),
                    params: Vec::new(),
                    params_fields: Vec::new(),
                    params_struct: params_struct_name(interface_name, &names.rust),
                    ret,
                    rpc_name: rpc_method_name(module_path, interface_name, &names.raw),
                    args: Vec::new(),
                }
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let name = rust_ident(&decl.0);
                        let raw_name = decl.0.clone();
                        let ret = attr_return_type(&spec.ty);
                        let param = render_param_type(&spec.ty, None);
                        out.push(MethodContext {
                            name: name.clone(),
                            params: Vec::new(),
                            params_fields: Vec::new(),
                            params_struct: params_struct_name(interface_name, &name),
                            ret,
                            rpc_name: rpc_method_name(module_path, interface_name, &raw_name),
                            args: Vec::new(),
                        });
                        let raw_setter = format!("set_{raw_name}");
                        let setter = rust_ident(&raw_setter);
                        out.push(MethodContext {
                            name: setter.clone(),
                            params: vec![format!("value: {param}")],
                            params_fields: vec![ParamField {
                                name: "value".to_string(),
                                ty: param,
                            }],
                            params_struct: params_struct_name(interface_name, &setter),
                            ret: "()".to_string(),
                            rpc_name: rpc_method_name(module_path, interface_name, &raw_setter),
                            args: vec!["value".to_string()],
                        });
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let name = rust_ident(&declarator.0);
                    let raw_name = declarator.0.clone();
                    let ret = attr_return_type(&spec.ty);
                    let param = render_param_type(&spec.ty, None);
                    out.push(MethodContext {
                        name: name.clone(),
                        params: Vec::new(),
                        params_fields: Vec::new(),
                        params_struct: params_struct_name(interface_name, &name),
                        ret,
                        rpc_name: rpc_method_name(module_path, interface_name, &raw_name),
                        args: Vec::new(),
                    });
                    let raw_setter = format!("set_{raw_name}");
                    let setter = rust_ident(&raw_setter);
                    out.push(MethodContext {
                        name: setter.clone(),
                        params: vec![format!("value: {param}")],
                        params_fields: vec![ParamField {
                            name: "value".to_string(),
                            ty: param,
                        }],
                        params_struct: params_struct_name(interface_name, &setter),
                        ret: "()".to_string(),
                        rpc_name: rpc_method_name(module_path, interface_name, &raw_setter),
                        args: vec!["value".to_string()],
                    });
                }
            }
            out
        }
    }
}

struct AttrNames {
    raw: String,
    rust: String,
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw: decl.0.clone(),
            rust: rust_ident(&decl.0),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    jsonrpc_type(ty)
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    let _ = attr;
    jsonrpc_type(ty)
}

fn params_struct_name(interface_name: &str, method_name: &str) -> String {
    let method_name = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}Params",
        rust_ident(interface_name),
        to_camel_case(method_name)
    )
}

fn jsonrpc_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "f64".to_string(),
            hir::SimpleTypeSpec::CharType => "char".to_string(),
            hir::SimpleTypeSpec::WideCharType => "char".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ObjectType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ValueBaseType => "serde_json::Value".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => render_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                format!("Vec<{}>", jsonrpc_type(&seq.ty))
            }
            hir::TemplateTypeSpec::StringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::WideStringType(_) => "String".to_string(),
            hir::TemplateTypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!(
                    "BTreeMap<{}, {}>",
                    jsonrpc_type(&map.key),
                    jsonrpc_type(&map.value)
                )
            }
        },
    }
}

fn render_scoped_name(value: &hir::ScopedName) -> String {
    let path = value
        .name
        .iter()
        .map(|part| rust_ident(part))
        .collect::<Vec<_>>()
        .join("::");
    if value.is_root {
        format!("::{path}")
    } else {
        path
    }
}

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar => "u8".to_string(),
        hir::IntegerType::U8 => "u8".to_string(),
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

fn to_camel_case(value: &str) -> String {
    let mut out = String::new();
    let mut uppercase = true;
    for ch in value.chars() {
        if ch == '_' {
            uppercase = true;
            continue;
        }
        if uppercase {
            out.push(ch.to_ascii_uppercase());
            uppercase = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn rpc_method_name(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
