use crate::error::IdlcResult;
use crate::generate::rust::util::rust_passthrough_attrs_from_annotations;
use std::collections::HashSet;
use xidl_parser::hir;

use super::interface_annotations::{has_annotation, validate_attr_collision};
use super::interface_attr_support::{
    attr_getter_method, attr_setter_method, attr_stream_method, attr_watch_method,
    readonly_attr_names,
};
use super::interface_model::RenderedAttr;

pub(super) fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
) -> IdlcResult<RenderedAttr> {
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    let rust_attrs = rust_passthrough_attrs_from_annotations(&attr.annotations);
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => render_readonly_attr(
            spec,
            interface_name,
            module_path,
            user_ops,
            emit_watch,
            &rust_attrs,
        ),
        hir::AttrDclInner::AttrSpec(spec) => render_mutable_attr(
            spec,
            interface_name,
            module_path,
            user_ops,
            emit_watch,
            &rust_attrs,
        ),
    }
}

fn render_readonly_attr(
    spec: &hir::ReadonlyAttrSpec,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
    emit_watch: bool,
    rust_attrs: &[String],
) -> IdlcResult<RenderedAttr> {
    let mut out = Vec::new();
    let mut watch_methods = Vec::new();
    for names in readonly_attr_names(spec) {
        validate_attr_collision(user_ops, &names.raw_attr, &names.raw_getter, "")?;
        out.push(attr_getter_method(
            &names.raw_attr,
            &names.raw_getter,
            &spec.ty,
            interface_name,
            module_path,
            rust_attrs,
        ));
        if emit_watch {
            out.push(attr_stream_method(
                &names.raw_attr,
                &spec.ty,
                interface_name,
                module_path,
                rust_attrs,
            ));
            watch_methods.push(attr_watch_method(
                &names.raw_attr,
                &names.raw_getter,
                &spec.ty,
                interface_name,
                module_path,
            ));
        }
    }
    Ok(RenderedAttr {
        methods: out,
        watch_methods,
    })
}

fn render_mutable_attr(
    spec: &hir::AttrSpec,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<&str>,
    emit_watch: bool,
    rust_attrs: &[String],
) -> IdlcResult<RenderedAttr> {
    let mut out = Vec::new();
    let mut watch_methods = Vec::new();
    let names = match &spec.declarator {
        hir::AttrDeclarator::SimpleDeclarator(list) => {
            list.iter().map(|decl| decl.0.clone()).collect::<Vec<_>>()
        }
        hir::AttrDeclarator::WithRaises { declarator, .. } => vec![declarator.0.clone()],
    };

    for raw_name in names {
        let raw_getter = format!("get_attribute_{raw_name}");
        let raw_setter = format!("set_attribute_{raw_name}");
        validate_attr_collision(user_ops, &raw_name, &raw_getter, &raw_setter)?;
        out.push(attr_getter_method(
            &raw_name,
            &raw_getter,
            &spec.ty,
            interface_name,
            module_path,
            rust_attrs,
        ));
        out.push(attr_setter_method(
            &raw_name,
            &raw_setter,
            &spec.ty,
            interface_name,
            module_path,
            rust_attrs,
        ));
        if emit_watch {
            out.push(attr_stream_method(
                &raw_name,
                &spec.ty,
                interface_name,
                module_path,
                rust_attrs,
            ));
            watch_methods.push(attr_watch_method(
                &raw_name,
                &raw_getter,
                &spec.ty,
                interface_name,
                module_path,
            ));
        }
    }
    Ok(RenderedAttr {
        methods: out,
        watch_methods,
    })
}
