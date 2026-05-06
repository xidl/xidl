mod spec_binding;
mod spec_context;
mod spec_meta;
mod spec_render;
mod spec_types;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::python_rest::PythonRestRenderer;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;
use xidl_parser::hir;
use xidl_parser::rest_hir::semantics::HttpStreamKind;
use xidl_parser::rest_hir::{HttpOperation, HttpOperationSource, RestHirDocument};

use self::spec_context::{ParamSource, build_method};
use self::spec_render::{render_endpoint_helper, render_method_types, render_route_builder};
use self::spec_types::{py_field_name, py_type_name};

#[derive(Serialize)]
struct PythonRestSpecTemplate {
    module_name: String,
    imports: PythonRestImports,
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct PythonRestImports {
    read_json_value: bool,
}

pub(crate) fn render_spec(
    spec: &hir::Specification,
    module_name: &str,
    rest_hir: &RestHirDocument,
) -> IdlcResult<String> {
    let renderer = PythonRestRenderer::new()?;
    let mut blocks = Vec::new();
    let mut imports = PythonRestImports {
        read_json_value: false,
    };
    render_definitions(&mut blocks, &spec.0, &[], rest_hir, &mut imports)?;
    renderer.render_template(
        "spec.py.j2",
        &PythonRestSpecTemplate {
            module_name: module_name.to_string(),
            imports,
            blocks,
        },
    )
}

fn render_definitions(
    out: &mut Vec<String>,
    defs: &[hir::Definition],
    module_path: &[String],
    rest_hir: &RestHirDocument,
    imports: &mut PythonRestImports,
) -> IdlcResult<()> {
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                render_definitions(out, &module.definition, &next, rest_hir, imports)?;
            }
            hir::Definition::InterfaceDcl(interface) => {
                render_interface(out, interface, module_path, rest_hir, imports)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_interface(
    out: &mut Vec<String>,
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    rest_hir: &RestHirDocument,
    imports: &mut PythonRestImports,
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
        return Ok(());
    };
    let Some(http_interface) = rest_hir.find_interface(module_path, &def.header.ident) else {
        return Ok(());
    };

    let interface_name = py_type_name(&def.header.ident);
    let methods = http_interface
        .operations
        .iter()
        .filter(|operation| !skips_operation(operation))
        .map(|operation| build_method(operation, &interface_name))
        .collect::<IdlcResult<Vec<_>>>()?;

    imports.read_json_value |= methods.iter().any(method_uses_read_json_value);

    validate_route_bindings(&methods)?;

    let mut block = String::new();
    for method in &methods {
        render_method_types(&mut block, method)?;
    }

    writeln!(block, "class {}Service(abc.ABC):", interface_name).unwrap();
    if methods.is_empty() {
        writeln!(block, "    pass").unwrap();
    } else {
        for method in &methods {
            writeln!(block, "    @abc.abstractmethod").unwrap();
            writeln!(
                block,
                "    async def {}(self, request: {}) -> {}:",
                method.method_name, method.request_type, method.response_type
            )
            .unwrap();
            writeln!(block, "        raise NotImplementedError").unwrap();
            writeln!(block).unwrap();
        }
    }

    for method in &methods {
        render_endpoint_helper(&mut block, &interface_name, method)?;
        render_route_builder(&mut block, &interface_name, method)?;
    }

    writeln!(
        block,
        "def {}_routes(service: {}Service) -> list[Route]:",
        py_field_name(&def.header.ident),
        interface_name
    )
    .unwrap();
    if methods.is_empty() {
        writeln!(block, "    return []").unwrap();
    } else {
        writeln!(block, "    return [").unwrap();
        for method in &methods {
            writeln!(block, "        {}(service),", method.route_builder_name).unwrap();
        }
        writeln!(block, "    ]").unwrap();
    }
    writeln!(block).unwrap();
    out.push(block);
    Ok(())
}

fn method_uses_read_json_value(method: &spec_context::MethodContext) -> bool {
    method
        .request_params
        .iter()
        .any(|param| matches!(param.source, ParamSource::Body) && param.flatten)
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
        HttpOperationSource::AttributeGet
            | HttpOperationSource::AttributeSet
            | HttpOperationSource::AttributeWatch
    )
}

pub(super) fn is_server_stream(kind: Option<HttpStreamKind>) -> bool {
    matches!(kind, Some(HttpStreamKind::Server))
}
