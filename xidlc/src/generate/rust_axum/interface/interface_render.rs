use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_axum::RustAxumRenderOutput;
use crate::generate::rust_axum::interface::RenderEnv;
use crate::generate::rust_axum::interface::interface_attr::render_attr_operation_from_http;
use crate::generate::rust_axum::interface::interface_http::{
    attr_operation_names, security_context,
};
use crate::generate::rust_axum::interface::interface_method::render_op_from_http;
use crate::generate::rust_axum::transport::TransportTracker;
use serde_json::json;
use xidl_parser::hir;
use xidl_parser::rest_hir::HttpOperationSource;
use xidl_parser::rest_hir::semantics::effective_security_with_origin;

pub(crate) fn render_interface_def(
    def: &hir::InterfaceDef,
    interface_annotations: &[hir::Annotation],
    env: RenderEnv<'_>,
) -> IdlcResult<RustAxumRenderOutput> {
    let mut out = RustAxumRenderOutput::default();
    let mut methods = Vec::new();
    let mut transport = TransportTracker::new(&def.header.ident);
    let rest_hir = env.renderer.rest_hir()?;
    let Some(http_interface) = rest_hir.find_interface(env.module_path, &def.header.ident) else {
        return Ok(out);
    };

    if let Some(body) = &def.interface_body {
        for operation in &http_interface.operations {
            let method = match operation.meta.source {
                HttpOperationSource::Method => {
                    let op = body
                        .0
                        .iter()
                        .find_map(|export| match export {
                            hir::Export::OpDcl(op) if op.ident == operation.meta.name => Some(op),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            IdlcError::rpc(format!(
                                "missing source operation '{}' for rust-axum rendering",
                                operation.meta.name
                            ))
                        })?;
                    render_op_from_http(op, operation, &def.header.ident, env, &mut transport)?
                }
                HttpOperationSource::AttributeGet
                | HttpOperationSource::AttributeSet
                | HttpOperationSource::AttributeWatch => {
                    let attr = body.0.iter().find_map(|export| match export {
                        hir::Export::AttrDcl(attr)
                            if attr_operation_names(attr).contains(&operation.meta.name) =>
                        {
                            Some(attr)
                        }
                        _ => None,
                    });
                    render_attr_operation_from_http(
                        attr,
                        operation,
                        &def.header.ident,
                        env,
                        &mut transport,
                    )?
                }
            };
            methods.push(method);
        }
    }

    let transport_modules = transport.render_modules(env.registry, env.module_path)?;
    let interface_security_profile =
        effective_security_with_origin(interface_annotations, &[]).map_err(IdlcError::rpc)?;
    let interface_security = security_context(&interface_security_profile);
    let has_interface_security = interface_security.has_basic_auth
        || interface_security.has_bearer_auth
        || !interface_security.api_key_requirements.is_empty();
    let interface_auth_ty = interface_security.auth_ty;

    // The version this interface's module pins: the document version for
    // per-version output, otherwise the enclosing version module's value.
    let api_version = rest_hir.document.version.clone().or_else(|| {
        let module = env.module_path.last()?;
        rest_hir.document.version_values.get(module).cloned()
    });
    let ctx = json!({
        "metadata": rest_hir.document,
        "api_version": api_version,
        "ident": rust_ident(&def.header.ident),
        "methods": methods,
        "rust_attrs": rust_passthrough_attrs_from_annotations(interface_annotations),
        "inbound_transport": transport_modules.inbound,
        "outbound_transport": transport_modules.outbound,
        "has_interface_security": has_interface_security,
        "interface_auth_ty": interface_auth_ty,
    });
    out.source
        .push(env.renderer.render_template("interface.rs.j2", &ctx)?);
    Ok(out)
}
