mod spec_binding;
mod spec_context;
mod spec_meta;
mod spec_render;
mod spec_types;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::http_hir::semantics::HttpStreamKind;
use crate::generate::http_hir::{HttpHirDocument, HttpOperation};
use crate::generate::python_http::PythonHttpRenderer;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;
use xidl_parser::hir;

use self::spec_context::build_method;
use self::spec_render::{render_endpoint_helper, render_method_types, render_route_builder};
use self::spec_types::{py_field_name, py_type_name};

#[derive(Serialize)]
struct PythonHttpSpecTemplate {
    module_name: String,
    body: String,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    http_hir: &HttpHirDocument,
) -> IdlcResult<String> {
    let renderer = PythonHttpRenderer::new()?;
    let mut body = String::new();
    render_definitions(&mut body, &spec.0, &[], http_hir)?;
    renderer.render_template(
        "spec.py.j2",
        &PythonHttpSpecTemplate {
            module_name: module_name.to_string(),
            body,
        },
    )
}

fn render_definitions(
    out: &mut String,
    defs: &[hir::Definition],
    module_path: &[String],
    http_hir: &HttpHirDocument,
) -> IdlcResult<()> {
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                render_definitions(out, &module.definition, &next, http_hir)?;
            }
            hir::Definition::InterfaceDcl(interface) => {
                render_interface(out, interface, module_path, http_hir)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_interface(
    out: &mut String,
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    http_hir: &HttpHirDocument,
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(());
    };

    let interface_name = py_type_name(&def.header.ident);
    let methods = http_interface
        .operations
        .iter()
        .filter(|operation| !skips_operation(operation))
        .map(|operation| build_method(operation, &interface_name))
        .collect::<IdlcResult<Vec<_>>>()?;

    validate_route_bindings(&methods)?;

    for method in &methods {
        render_method_types(out, method)?;
    }

    writeln!(out, "class {}Service(abc.ABC):", interface_name).unwrap();
    if methods.is_empty() {
        writeln!(out, "    pass").unwrap();
    } else {
        for method in &methods {
            writeln!(out, "    @abc.abstractmethod").unwrap();
            writeln!(
                out,
                "    async def {}(self, request: {}) -> {}:",
                method.method_name, method.request_type, method.response_type
            )
            .unwrap();
            writeln!(out, "        raise NotImplementedError").unwrap();
            writeln!(out).unwrap();
        }
    }

    for method in &methods {
        render_endpoint_helper(out, &interface_name, method)?;
        render_route_builder(out, &interface_name, method)?;
    }

    writeln!(
        out,
        "def {}_routes(service: {}Service) -> list[Route]:",
        py_field_name(&def.header.ident),
        interface_name
    )
    .unwrap();
    if methods.is_empty() {
        writeln!(out, "    return []").unwrap();
    } else {
        writeln!(out, "    return [").unwrap();
        for method in &methods {
            writeln!(out, "        {}(service),", method.route_builder_name).unwrap();
        }
        writeln!(out, "    ]").unwrap();
    }
    writeln!(out).unwrap();
    Ok(())
}

fn validate_route_bindings(methods: &[spec_context::MethodContext]) -> IdlcResult<()> {
    let mut route_bindings = HashMap::<String, String>::new();
    for method in methods {
        for path in &method.paths {
            let key = format!("{} {}", method.http_method, path);
            if let Some(previous) = route_bindings.insert(key.clone(), method.raw_name.clone()) {
                return Err(IdlcError::rpc(format!(
                    "duplicate HTTP route binding: {key} (methods: {previous}, {})",
                    method.raw_name
                )));
            }
        }
    }
    Ok(())
}

fn skips_operation(operation: &HttpOperation) -> bool {
    matches!(
        operation.source,
        crate::generate::http_hir::HttpOperationSource::AttributeGet
            | crate::generate::http_hir::HttpOperationSource::AttributeSet
            | crate::generate::http_hir::HttpOperationSource::AttributeWatch
    )
}

pub(super) fn is_server_stream(kind: Option<HttpStreamKind>) -> bool {
    matches!(kind, Some(HttpStreamKind::Server))
}
