use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use std::collections::HashSet;
use xidl_parser::hir;

use super::interface_attr::render_attr;
use super::interface_ops::render_op;

pub(super) fn render_interface_def(
    def: &hir::InterfaceDef,
    interface_annotations: &[hir::Annotation],
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<JsonRpcRenderOutput> {
    let mut out = JsonRpcRenderOutput::default();
    let mut methods = Vec::new();
    let mut watch_methods = Vec::new();
    let user_ops = collect_user_ops(def);

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.extend(render_op(op, &def.header.ident, module_path)?);
                }
                hir::Export::AttrDcl(attr) => {
                    let rendered = render_attr(attr, &def.header.ident, module_path, &user_ops)?;
                    methods.extend(rendered.methods);
                    watch_methods.extend(rendered.watch_methods);
                }
                _ => {}
            }
        }
    }

    let bidi_method_names = methods
        .iter()
        .filter(|method| matches!(method.kind.as_str(), "stream_op" | "stream_source"))
        .map(|method| method.rpc_name.clone())
        .collect::<Vec<_>>();

    let ctx = serde_json::json!({
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
        "bidi_method_names": bidi_method_names,
        "watch_methods": watch_methods,
        "rust_attrs": rust_passthrough_attrs_from_annotations(interface_annotations),
    });
    let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
    out.source.push(rendered);
    Ok(out)
}

fn collect_user_ops(def: &hir::InterfaceDef) -> HashSet<&str> {
    let mut user_ops = HashSet::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            if let hir::Export::OpDcl(op) = export {
                user_ops.insert(op.ident.as_str());
            }
        }
    }
    user_ops
}
