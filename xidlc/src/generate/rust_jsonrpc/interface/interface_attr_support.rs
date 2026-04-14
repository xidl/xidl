use crate::generate::rust::util::rust_ident;
use xidl_parser::hir;

use super::interface_model::{
    AttrNames, MethodContext, OutputField, ParamField, WatchMethodContext,
};
use super::interface_names::{params_struct_name, response_struct_name, rpc_method_name};
use super::interface_types::{attr_return_type, render_param_type};

pub(super) fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw_attr: decl.0.clone(),
            raw_getter: format!("get_attribute_{}", decl.0),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

pub(super) fn attr_getter_method(
    _raw_name: &str,
    raw_getter: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    rust_attrs: &[String],
) -> MethodContext {
    let getter = rust_ident(raw_getter);
    MethodContext {
        kind: "rpc".to_string(),
        stream_mode: String::new(),
        name: getter.clone(),
        rust_attrs: rust_attrs.to_vec(),
        params: Vec::new(),
        params_fields: Vec::new(),
        params_struct: params_struct_name(interface_name, &getter),
        ret: attr_return_type(ty),
        rpc_name: rpc_method_name(module_path, interface_name, raw_getter),
        args: Vec::new(),
        response_kind: "single_return".to_string(),
        response_struct: response_struct_name(interface_name, &getter),
        response_fields: vec![OutputField {
            name: rust_ident("return"),
            json_name: "return".to_string(),
            ty: attr_return_type(ty),
        }],
        response_single_field: rust_ident("return"),
        stream_item_ty: String::new(),
    }
}

pub(super) fn attr_setter_method(
    raw_name: &str,
    raw_setter: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    rust_attrs: &[String],
) -> MethodContext {
    let setter = rust_ident(raw_setter);
    let param = render_param_type(ty, None);
    let value_ident = rust_ident(raw_name);
    MethodContext {
        kind: "rpc".to_string(),
        stream_mode: String::new(),
        name: setter.clone(),
        rust_attrs: rust_attrs.to_vec(),
        params: vec![format!("{value_ident}: {param}")],
        params_fields: vec![ParamField {
            name: value_ident.clone(),
            ty: param,
        }],
        params_struct: params_struct_name(interface_name, &setter),
        ret: "()".to_string(),
        rpc_name: rpc_method_name(module_path, interface_name, raw_setter),
        args: vec![value_ident],
        response_kind: "empty".to_string(),
        response_struct: response_struct_name(interface_name, &setter),
        response_fields: Vec::new(),
        response_single_field: String::new(),
        stream_item_ty: String::new(),
    }
}

pub(super) fn attr_stream_method(
    raw_name: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
    rust_attrs: &[String],
) -> MethodContext {
    let raw_stream_setter = format!("set_attribute_{raw_name}");
    MethodContext {
        kind: "stream_source".to_string(),
        stream_mode: String::new(),
        name: rust_ident(&raw_stream_setter),
        rust_attrs: rust_attrs.to_vec(),
        params: Vec::new(),
        params_fields: Vec::new(),
        params_struct: String::new(),
        ret: String::new(),
        rpc_name: rpc_method_name(module_path, interface_name, &raw_stream_setter),
        args: Vec::new(),
        response_kind: "empty".to_string(),
        response_struct: String::new(),
        response_fields: Vec::new(),
        response_single_field: String::new(),
        stream_item_ty: attr_return_type(ty),
    }
}

pub(super) fn attr_watch_method(
    raw_name: &str,
    raw_getter: &str,
    ty: &hir::TypeSpec,
    interface_name: &str,
    module_path: &[String],
) -> WatchMethodContext {
    WatchMethodContext {
        getter_name: rust_ident(raw_getter),
        item_ty: attr_return_type(ty),
        stream_rpc_name: rpc_method_name(
            module_path,
            interface_name,
            &format!("set_attribute_{raw_name}"),
        ),
    }
}
