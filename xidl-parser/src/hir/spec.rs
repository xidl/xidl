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
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        Self::lower(value, expand_interface(&properties))
    }

    pub fn from_typed_ast_with_path(
        value: crate::typed_ast::Specification,
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<Self> {
        Self::lower(value, true)
    }

    pub fn project_typed_ast_with_properties_and_path(
        value: crate::typed_ast::Specification,
        properties: ParserProperties,
        _path: impl AsRef<Path>,
    ) -> crate::error::ParserResult<ProjectedHir> {
        let spec = Self::lower(value, expand_interface(&properties))?;
        match hir_projection_kind(&properties) {
            HirProjectionKind::Rpc => Ok(ProjectedHir::Rpc(spec)),
            HirProjectionKind::Http => rest_hir::project(&spec).map(ProjectedHir::Http),
            HirProjectionKind::JsonRpc => jsonrpc_hir::project(&spec).map(ProjectedHir::JsonRpc),
        }
    }

    fn lower(
        value: crate::typed_ast::Specification,
        expand_interfaces: bool,
    ) -> crate::error::ParserResult<Self> {
        let mut definitions = Vec::new();
        collect_defs(value.0, &mut definitions);
        let mut spec = Self(definitions);
        // All source targets share HTTP declaration validation; builtins belong
        // to its temporary type environment, not the generated user models.
        rest_hir::HttpUnion::validate_source(&spec.0)?;
        // Validate lexical source scopes before adding RPC wrapper declarations
        // at their generated scopes.
        if expand_interfaces {
            Self::expand_interfaces(&mut spec.0, &mut Vec::new())?;
        }
        semantic::analyze(&mut spec);
        Ok(spec)
    }

    fn expand_interfaces(
        definitions: &mut Vec<Definition>,
        modules: &mut Vec<String>,
    ) -> crate::error::ParserResult<()> {
        for mut definition in std::mem::take(definitions) {
            match &mut definition {
                Definition::ModuleDcl(module) => {
                    modules.push(module.ident.clone());
                    Self::expand_interfaces(&mut module.definition, modules)?;
                    modules.pop();
                }
                Definition::InterfaceDcl(interface) => {
                    definitions.extend(interface_codegen::expand_interface(interface, modules)?);
                }
                _ => {}
            }
            definitions.push(definition);
        }
        Ok(())
    }
}

pub(crate) fn spec_from_typed_ast(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
) -> Specification {
    Specification::lower(value, expand_interfaces).expect("HIR conversion should not fail")
}

pub(super) fn collect_defs(defs: Vec<crate::typed_ast::Definition>, out: &mut Vec<Definition>) {
    for def in defs {
        match def {
            crate::typed_ast::Definition::ModuleDcl(module) => {
                let ident = module.ident.0;
                let annotations = expand_annotations(module.annotations);
                let mut inner = Vec::new();
                collect_defs(module.definition, &mut inner);
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
                out.push(Definition::InterfaceDcl(InterfaceDcl::from(value)));
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
