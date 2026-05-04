use crate::error::IdlcResult;
use crate::generate::rust_jsonrpc::JsonRpcRenderer;
use crate::generate::rust_jsonrpc::interface::render_interface_with_path;
use xidl_parser::jsonrpc_hir::JsonRpcInterface;

pub(crate) fn render_module_body(
    interfaces: &[JsonRpcInterface],
    renderer: &JsonRpcRenderer,
) -> IdlcResult<Vec<String>> {
    render_module_body_with_path(interfaces, renderer, &[])
}

fn render_module_body_with_path(
    interfaces: &[JsonRpcInterface],
    renderer: &JsonRpcRenderer,
    module_path: &[String],
) -> IdlcResult<Vec<String>> {
    let mut out = Vec::new();
    let mut module_order = Vec::new();

    for interface in interfaces
        .iter()
        .filter(|item| item.module_path == module_path)
    {
        let rendered = render_interface_with_path(interface, renderer)?;
        out.extend(rendered.source);
    }

    for interface in interfaces {
        if interface.module_path.len() <= module_path.len()
            || interface.module_path[..module_path.len()] != *module_path
        {
            continue;
        }
        let next = interface.module_path[module_path.len()].clone();
        if !module_order.contains(&next) {
            module_order.push(next);
        }
    }

    for name in module_order {
        let mut next_path = module_path.to_vec();
        next_path.push(name.clone());
        let definitions = render_module_body_with_path(interfaces, renderer, &next_path)?;
        let rendered = renderer.render_template(
            "module.rs.j2",
            &serde_json::json!({
                "ident": crate::generate::rust::util::rust_ident(&name),
                "definitions": &definitions,
            }),
        )?;
        out.push(rendered);
    }

    Ok(out)
}
