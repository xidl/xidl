use super::{
    Definition, InterfaceDcl, ModuleDcl, ParserProperties, Specification, TypeDcl,
    expand_annotations, include, interface_codegen, parse_xidlc_pragma,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

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
        None,
        None,
    )
    .expect("pathless HIR conversion should not fail");
    Specification(definitions)
}

fn spec_from_typed_ast_with_path(
    value: crate::typed_ast::Specification,
    expand_interfaces: bool,
    path: &Path,
) -> crate::error::ParserResult<Specification> {
    let root = include::normalize_path(path);
    let mut definitions = Vec::new();
    let mut include_stack = vec![root.clone()];
    collect_defs_with_context(
        value.0,
        &mut Vec::new(),
        expand_interfaces,
        &mut definitions,
        Some(root.as_path()),
        Some(&mut include_stack),
    )?;
    Ok(Specification(definitions))
}

fn collect_defs_with_context(
    defs: Vec<crate::typed_ast::Definition>,
    modules: &mut Vec<String>,
    expand_interfaces: bool,
    out: &mut Vec<Definition>,
    current_file: Option<&Path>,
    mut include_stack: Option<&mut Vec<PathBuf>>,
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
                    current_file,
                    include_stack.as_deref_mut(),
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
            crate::typed_ast::Definition::PreprocInclude(include_def) => {
                let Some(current_file) = current_file else {
                    continue;
                };
                let path = include::resolve_include_path(current_file, &include_def)?;
                let typed = parse_included_specification(&path)?;
                let stack = include_stack
                    .as_deref_mut()
                    .expect("include stack must exist when current file path is set");
                guard_include_cycle(stack, &path)?;
                stack.push(path.clone());
                collect_defs_with_context(
                    typed.0,
                    modules,
                    expand_interfaces,
                    out,
                    Some(path.as_path()),
                    Some(stack),
                )?;
                stack.pop();
            }
            crate::typed_ast::Definition::TemplateModuleDcl(_)
            | crate::typed_ast::Definition::TemplateModuleInst(_)
            | crate::typed_ast::Definition::PreprocDefine(_) => {}
        }
    }

    Ok(())
}

fn expand_interface(properties: &ParserProperties) -> bool {
    properties
        .get("expand_interface")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_included_specification(
    path: &Path,
) -> crate::error::ParserResult<crate::typed_ast::Specification> {
    let source = fs::read_to_string(path).map_err(|err| {
        crate::error::ParseError::Message(format!(
            "failed to read include '{}': {err}",
            path.display()
        ))
    })?;
    crate::parser::parser_text(&source).map_err(|err| {
        crate::error::ParseError::Message(format!(
            "failed to parse include '{}': {err}",
            path.display()
        ))
    })
}

fn guard_include_cycle(stack: &[PathBuf], path: &Path) -> crate::error::ParserResult<()> {
    if stack.contains(&path.to_path_buf()) {
        let chain = stack
            .iter()
            .chain(std::iter::once(&path.to_path_buf()))
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(crate::error::ParseError::Message(format!(
            "cyclic include detected: {chain}"
        )));
    }
    Ok(())
}
