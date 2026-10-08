mod interface;
mod merged;
mod render;
mod transport;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::macros::hashmap;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub use render::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};

pub fn generate(
    rest_hir: xidl_parser::rest_hir::RestHirDocument,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let spec = rest_hir.spec.clone();
    let file_name = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            crate::error::IdlcError::rpc(format!(
                "rust-axum generator requires a valid input file stem for '{}'",
                input_path.display()
            ))
        })?;
    let filename = format!("{file_name}.rs");

    let mut renderer = RustAxumRenderer::new()?;
    renderer.extend(&props, rest_hir.clone());

    let reachable = collect_reachable_types(&rest_hir);
    let refs = spec.0.iter().collect::<Vec<_>>();
    let registry = transport::build_type_registry(&refs, &[]);
    let rust_renderer = crate::generate::rust::RustRenderer::new(std::collections::HashMap::new())?;
    let builder = merged::MergedAxumSpecBuilder::new(&renderer, rust_renderer, registry, reachable);
    let definitions = builder.render(&spec)?;

    let content = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": definitions,
        }),
    )?;

    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })])
}

fn collect_reachable_types(
    rest_hir: &xidl_parser::rest_hir::RestHirDocument,
) -> std::collections::HashSet<String> {
    let mut reachable = std::collections::HashSet::new();
    let mut registry = std::collections::HashMap::new();
    collect_registry(&rest_hir.spec.0, &[], &mut registry);

    for interface in &rest_hir.interfaces {
        for op in &interface.operations {
            for param in &op.signature.params {
                mark_reachable(&param.ty, &mut reachable, &registry, &interface.module_path);
            }
            if let Some(ret) = &op.signature.return_type {
                mark_reachable(ret, &mut reachable, &registry, &interface.module_path);
            }
        }
    }

    // Include constants and exceptions as roots
    for def in &rest_hir.spec.0 {
        collect_extra_roots(def, &mut reachable, &registry, &[]);
    }

    reachable
}

fn collect_extra_roots(
    def: &hir::Definition,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    match def {
        hir::Definition::ModuleDcl(m) => {
            let mut next = scope.to_vec();
            next.push(m.ident.clone());
            for def in &m.definition {
                collect_extra_roots(def, reachable, registry, &next);
            }
        }
        hir::Definition::ConstDcl(c) => {
            mark_reachable_const(&c.ty, reachable, registry, scope);
        }
        hir::Definition::ExceptDcl(e) => {
            for m in &e.member {
                mark_reachable(&m.ty, reachable, registry, scope);
            }
        }
        _ => {}
    }
}

fn collect_registry<'a>(
    defs: &'a [hir::Definition],
    path: &[String],
    registry: &mut std::collections::HashMap<String, &'a hir::Definition>,
) {
    for def in defs {
        match def {
            hir::Definition::ModuleDcl(m) => {
                let mut next = path.to_vec();
                next.push(m.ident.clone());
                collect_registry(&m.definition, &next, registry);
            }
            _ => {
                for ident in get_def_idents(def) {
                    let mut full = path.to_vec();
                    full.push(ident);
                    registry.insert(full.join("::"), def);
                }
            }
        }
    }
}

pub(crate) fn get_def_idents(def: &hir::Definition) -> Vec<String> {
    match def {
        hir::Definition::ConstrTypeDcl(c) => match c {
            hir::ConstrTypeDcl::StructDcl(s) => vec![s.ident.clone()],
            hir::ConstrTypeDcl::EnumDcl(e) => vec![e.ident.clone()],
            hir::ConstrTypeDcl::UnionDef(u) => vec![u.ident.clone()],
            hir::ConstrTypeDcl::BitsetDcl(b) => vec![b.ident.clone()],
            hir::ConstrTypeDcl::BitmaskDcl(b) => vec![b.ident.clone()],
            hir::ConstrTypeDcl::StructForwardDcl(s) => vec![s.ident.clone()],
            hir::ConstrTypeDcl::UnionForwardDcl(u) => vec![u.ident.clone()],
        },
        hir::Definition::TypeDcl(ty) => match ty {
            hir::TypeDcl::ConstrTypeDcl(c) => match c {
                hir::ConstrTypeDcl::StructDcl(s) => vec![s.ident.clone()],
                hir::ConstrTypeDcl::EnumDcl(e) => vec![e.ident.clone()],
                hir::ConstrTypeDcl::UnionDef(u) => vec![u.ident.clone()],
                hir::ConstrTypeDcl::BitsetDcl(b) => vec![b.ident.clone()],
                hir::ConstrTypeDcl::BitmaskDcl(b) => vec![b.ident.clone()],
                hir::ConstrTypeDcl::StructForwardDcl(s) => vec![s.ident.clone()],
                hir::ConstrTypeDcl::UnionForwardDcl(u) => vec![u.ident.clone()],
            },
            hir::TypeDcl::TypedefDcl(t) => t
                .decl
                .iter()
                .map(|d| match d {
                    hir::Declarator::SimpleDeclarator(s) => s.0.clone(),
                    hir::Declarator::ArrayDeclarator(a) => a.ident.clone(),
                })
                .collect(),
            _ => vec![],
        },
        hir::Definition::ConstDcl(c) => vec![c.ident.clone()],
        hir::Definition::ExceptDcl(e) => vec![e.ident.clone()],
        _ => vec![],
    }
}

fn mark_reachable(
    ty: &hir::TypeSpec,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    match ty {
        hir::TypeSpec::ScopedName(name) => {
            mark_reachable_scoped(name, reachable, registry, scope);
        }
        hir::TypeSpec::SequenceType(seq) => mark_reachable(&seq.ty, reachable, registry, scope),
        hir::TypeSpec::MapType(map) => {
            mark_reachable(&map.key, reachable, registry, scope);
            mark_reachable(&map.value, reachable, registry, scope);
        }
        hir::TypeSpec::TemplateType(tmpl) => {
            for arg in &tmpl.args {
                mark_reachable(arg, reachable, registry, scope);
            }
        }
        _ => {}
    }
}

fn mark_reachable_scoped(
    name: &hir::ScopedName,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    let canonical = crate::generate::utils::scope::resolve_canonical(scope, name, |key| {
        registry.contains_key(key)
    });
    if let Some(full) = canonical {
        if reachable.insert(full.clone()) {
            if let Some(def) = registry.get(full.as_str()) {
                let parent = parent_scope(&full);
                mark_reachable_from_def(def, reachable, registry, &parent);
            }
        }
    } else {
        reachable.insert(name.name.join("::"));
    }
}

fn parent_scope(canonical: &str) -> Vec<String> {
    let mut parts: Vec<String> = canonical.split("::").map(str::to_string).collect();
    parts.pop();
    parts
}

fn mark_reachable_const(
    ty: &hir::ConstType,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    match ty {
        hir::ConstType::ScopedName(name) => {
            mark_reachable_scoped(name, reachable, registry, scope);
        }
        hir::ConstType::SequenceType(seq) => mark_reachable(&seq.ty, reachable, registry, scope),
        _ => {}
    }
}

fn mark_reachable_from_def(
    def: &hir::Definition,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    match def {
        hir::Definition::TypeDcl(ty) => match ty {
            hir::TypeDcl::ConstrTypeDcl(c) => {
                mark_reachable_from_constr(c, reachable, registry, scope);
            }
            hir::TypeDcl::TypedefDcl(t) => {
                if let hir::TypedefType::TypeSpec(ty) = &t.ty {
                    mark_reachable(ty, reachable, registry, scope)
                }
            }
            _ => {}
        },
        hir::Definition::ConstrTypeDcl(c) => {
            mark_reachable_from_constr(c, reachable, registry, scope);
        }
        hir::Definition::ConstDcl(c) => mark_reachable_const(&c.ty, reachable, registry, scope),
        hir::Definition::ExceptDcl(e) => {
            for m in &e.member {
                mark_reachable(&m.ty, reachable, registry, scope);
            }
        }
        _ => {}
    }
}

fn mark_reachable_from_constr(
    c: &hir::ConstrTypeDcl,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
    scope: &[String],
) {
    match c {
        hir::ConstrTypeDcl::StructDcl(s) => {
            for m in &s.member {
                mark_reachable(&m.ty, reachable, registry, scope);
            }
        }
        hir::ConstrTypeDcl::UnionDef(u) => {
            if let hir::SwitchTypeSpec::ScopedName(name) = &u.switch_type_spec {
                mark_reachable_scoped(name, reachable, registry, scope);
            }
            for c in &u.case {
                match &c.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => {
                        mark_reachable(ty, reachable, registry, scope)
                    }
                    hir::ElementSpecTy::ConstrTypeDcl(c) => {
                        mark_reachable_from_constr(c, reachable, registry, scope)
                    }
                }
            }
        }
        hir::ConstrTypeDcl::BitsetDcl(b) => {
            if let Some(parent) = &b.parent {
                mark_reachable_scoped(parent, reachable, registry, scope);
            }
        }
        _ => {}
    }
}

pub(crate) struct RustAxumCodegen;

impl crate::jsonrpc::Codegen for RustAxumCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(hashmap! {
            "expand_interface" => false,
            "hir_kind" => "http",
            "enable_client" => true,
            "enable_server" => true,
            "enable_render_header" => true,
            "enable_serialize" => true,
            "enable_deserialize" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let rest_hir = input_hir.into_rest_hir();
        generate(rest_hir, Path::new(&path), props)
            .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))
    }
}
