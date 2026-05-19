use super::{
    Definition, InterfaceDcl, ModuleDcl, ParserProperties, Specification, TypeDcl,
    expand_annotations, interface_codegen, parse_xidlc_pragma,
};
use crate::jsonrpc_hir;
use crate::rest_hir::{self, HirProjectionKind, ProjectedHir};
use crate::semantic;
use serde_json::Value;
use std::path::Path;

impl From<crate::typed_ast::Specification> for Specification {
    fn from(value: crate::typed_ast::Specification) -> Self {
        spec_from_typed_ast(value, true)
    }
}

impl Specification {
    pub fn from_typed_ast_with_properties(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
    ) -> Self {
        spec_from_typed_ast(value, expand_interface(&properties))
    }

    pub fn from_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        spec_from_typed_ast_with_path(value, expand_interface(&properties), path.as_ref())
    }

    pub fn from_typed_ast_with_path(
        value: crate::typed_ast::Specification,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        spec_from_typed_ast_with_path(value, true, path.as_ref())
    }

    pub fn project_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<ProjectedHir> {
        let spec =
            spec_from_typed_ast_with_path(value, expand_interface(&properties), path.as_ref())?;
        match hir_projection_kind(&properties) {
            HirProjectionKind::Rpc => Ok(ProjectedHir::Rpc(spec)),
            HirProjectionKind::Http => rest_hir::project(&spec).map(ProjectedHir::Http),
            HirProjectionKind::JsonRpc => jsonrpc_hir::project(&spec).map(ProjectedHir::JsonRpc),
        }
    }
}

pub(crate) fn spec_from_typed_ast(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
) -> Specification {
    let mut definitions = Vec::new();
    collect_defs_with_context(
        value.0,
        &mut Vec::new(),
        expand_interfaces,
        &mut definitions,
    )
    .expect("HIR conversion should not fail");
    let mut spec = Specification(definitions);
    semantic::analyze(&mut spec);
    spec
}

fn spec_from_typed_ast_with_path(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
    _path: &Path,
) -> crate::error::ParserResult<Specification> {
    let mut definitions = Vec::new();
    collect_defs_with_context(
        value.0,
        &mut Vec::new(),
        expand_interfaces,
        &mut definitions,
    )?;
    let mut spec = Specification(definitions);
    semantic::analyze(&mut spec);
    Ok(spec)
}

fn collect_defs_with_context(
    defs: Vec<crate::typed_ast::Definition>,
    modules: &mut Vec<String>,
    expand_interfaces: bool,
    out: &mut Vec<Definition>,
) -> crate::error::ParserResult<()> {
    for def in defs {
        match def {
            crate::typed_ast::Definition::ModuleDcl(module) => {
                let ident = module.ident.0;
                let annotations = expand_annotations(module.annotations);
                modules.push(ident.clone());
                let mut inner = Vec::new();
                collect_defs_with_context(
                    module.definition,
                    modules,
                    expand_interfaces,
                    &mut inner,
                )?;
                modules.pop();
                out.push(Definition::ModuleDcl(ModuleDcl {
                    annotations,
                    ident,
                    definition: inner,
                }));
            }
            crate::typed_ast::Definition::PreprocCall(call) => {
                if let Some(pragma) = parse_xidlc_pragma(&call) {
                    out.push(Definition::Pragma(pragma));
                }
            }
            crate::typed_ast::Definition::TypeDcl(value) => {
                out.push(Definition::TypeDcl(TypeDcl::from(value)))
            }
            crate::typed_ast::Definition::ConstDcl(value) => {
                out.push(Definition::ConstDcl(value.into()))
            }
            crate::typed_ast::Definition::ExceptDcl(value) => {
                out.push(Definition::ExceptDcl(value.into()))
            }
            crate::typed_ast::Definition::InterfaceDcl(value) => {
                let interface = InterfaceDcl::from(value);
                if expand_interfaces {
                    let generated = interface_codegen::expand_interface(&interface, modules)
                        .unwrap_or_else(|err| panic!("interface expansion failed: {err}"));
                    out.extend(generated);
                }
                out.push(Definition::InterfaceDcl(interface));
            }
            crate::typed_ast::Definition::PreprocInclude(_) => {
                // Includes are now handled at the tree-sitter stage.
                // If we see a PreprocInclude here, it means it was not expanded.
            }
            crate::typed_ast::Definition::TemplateModuleDcl(_)
            | crate::typed_ast::Definition::TemplateModuleInst(_)
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }

    Ok(())
}

fn expand_interface(properties: &ParserProperties) -> bool {
    if let Some(expand) = properties.get("expand_interface").and_then(Value::as_bool) {
        return expand;
    }

    matches!(hir_projection_kind(properties), HirProjectionKind::Rpc)
}

fn hir_projection_kind(properties: &ParserProperties) -> HirProjectionKind {
    match properties.get("hir_kind").and_then(Value::as_str) {
        Some(value) if value.eq_ignore_ascii_case("http") => HirProjectionKind::Http,
        Some(value)
            if value.eq_ignore_ascii_case("jsonrpc") || value.eq_ignore_ascii_case("json-rpc") =>
        {
            HirProjectionKind::JsonRpc
        }
        _ => HirProjectionKind::Rpc,
    }
}
