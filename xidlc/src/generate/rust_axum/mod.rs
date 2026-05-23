mod definition;
mod interface;
mod render;
mod spec;
mod transport;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
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
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();
    let filename = format!("{file_name}.rs");

    let mut renderer = RustAxumRenderer::new()?;
    renderer.extend(&props, rest_hir.clone());
    let output = spec.render(&renderer)?;

    let content = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": output.source,
        }),
    )?;

    let mut artifacts = vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })];

    let reachable = collect_reachable_types(&rest_hir);
    let pruned_spec = prune_definitions(spec, &reachable);
    let non_interface = strip_interfaces(pruned_spec);
    if !non_interface.0.is_empty() {
        let props = hashmap! {
            "enable_render_header" => false,
            "enable_metadata" => false,
            "enable_serialize" => false,
            "enable_deserialize" => false
        };

        artifacts.push(Artifact::new_hir(ArtifactHir {
            lang: "rust".into(),
            hir: non_interface,
            props,
        }));
    }

    Ok(artifacts)
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
                mark_reachable(&param.ty, &mut reachable, &registry);
            }
            if let Some(ret) = &op.signature.return_type {
                mark_reachable(ret, &mut reachable, &registry);
            }
        }
    }

    // Include constants and exceptions as roots
    for def in &rest_hir.spec.0 {
        collect_extra_roots(def, &mut reachable, &registry);
    }

    reachable
}

fn collect_extra_roots(
    def: &hir::Definition,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
) {
    match def {
        hir::Definition::ModuleDcl(m) => {
            for def in &m.definition {
                collect_extra_roots(def, reachable, registry);
            }
        }
        hir::Definition::ConstDcl(c) => {
            mark_reachable_const(&c.ty, reachable, registry);
        }
        hir::Definition::ExceptDcl(e) => {
            for m in &e.member {
                mark_reachable(&m.ty, reachable, registry);
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

fn get_def_idents(def: &hir::Definition) -> Vec<String> {
    match def {
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
) {
    match ty {
        hir::TypeSpec::ScopedName(name) => {
            mark_reachable_scoped(name, reachable, registry);
        }
        hir::TypeSpec::SequenceType(seq) => mark_reachable(&seq.ty, reachable, registry),
        hir::TypeSpec::MapType(map) => {
            mark_reachable(&map.key, reachable, registry);
            mark_reachable(&map.value, reachable, registry);
        }
        hir::TypeSpec::TemplateType(tmpl) => {
            for arg in &tmpl.args {
                mark_reachable(arg, reachable, registry);
            }
        }
        _ => {}
    }
}

fn mark_reachable_scoped(
    name: &hir::ScopedName,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
) {
    let full = name.name.join("::");
    if reachable.insert(full.clone()) {
        if let Some(def) = registry.get(&full) {
            mark_reachable_from_def(def, reachable, registry);
        }
    }
}

fn mark_reachable_const(
    ty: &hir::ConstType,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
) {
    match ty {
        hir::ConstType::ScopedName(name) => {
            mark_reachable_scoped(name, reachable, registry);
        }
        hir::ConstType::SequenceType(seq) => mark_reachable(&seq.ty, reachable, registry),
        _ => {}
    }
}

fn mark_reachable_from_def(
    def: &hir::Definition,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
) {
    match def {
        hir::Definition::TypeDcl(ty) => match ty {
            hir::TypeDcl::ConstrTypeDcl(c) => {
                mark_reachable_from_constr(c, reachable, registry);
            }
            hir::TypeDcl::TypedefDcl(t) => {
                if let hir::TypedefType::TypeSpec(ty) = &t.ty {
                    mark_reachable(ty, reachable, registry)
                }
            }
            _ => {}
        },
        hir::Definition::ConstDcl(c) => mark_reachable_const(&c.ty, reachable, registry),
        hir::Definition::ExceptDcl(e) => {
            for m in &e.member {
                mark_reachable(&m.ty, reachable, registry);
            }
        }
        _ => {}
    }
}

fn mark_reachable_from_constr(
    c: &hir::ConstrTypeDcl,
    reachable: &mut std::collections::HashSet<String>,
    registry: &std::collections::HashMap<String, &hir::Definition>,
) {
    match c {
        hir::ConstrTypeDcl::StructDcl(s) => {
            for m in &s.member {
                mark_reachable(&m.ty, reachable, registry);
            }
        }
        hir::ConstrTypeDcl::UnionDef(u) => {
            if let hir::SwitchTypeSpec::ScopedName(name) = &u.switch_type_spec {
                mark_reachable_scoped(name, reachable, registry);
            }
            for c in &u.case {
                match &c.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => mark_reachable(ty, reachable, registry),
                    hir::ElementSpecTy::ConstrTypeDcl(c) => {
                        mark_reachable_from_constr(c, reachable, registry)
                    }
                }
            }
        }
        hir::ConstrTypeDcl::BitsetDcl(b) => {
            if let Some(parent) = &b.parent {
                mark_reachable_scoped(parent, reachable, registry);
            }
        }
        _ => {}
    }
}

fn prune_definitions(
    spec: hir::Specification,
    reachable: &std::collections::HashSet<String>,
) -> hir::Specification {
    hir::Specification(prune_defs_recursive(spec.0, reachable, &[]))
}

fn prune_defs_recursive(
    defs: Vec<hir::Definition>,
    reachable: &std::collections::HashSet<String>,
    path: &[String],
) -> Vec<hir::Definition> {
    let mut out = Vec::new();
    for def in defs {
        match &def {
            hir::Definition::ModuleDcl(m) => {
                let mut next = path.to_vec();
                next.push(m.ident.clone());
                let inner = prune_defs_recursive(m.definition.clone(), reachable, &next);
                if !inner.is_empty() {
                    out.push(hir::Definition::ModuleDcl(hir::ModuleDcl {
                        definition: inner,
                        ..m.clone()
                    }));
                }
            }
            hir::Definition::Pragma(_)
            | hir::Definition::InterfaceDcl(_)
            | hir::Definition::ConstDcl(_)
            | hir::Definition::ExceptDcl(_) => out.push(def),
            _ => {
                let idents = get_def_idents(&def);
                let mut is_reachable = false;
                for ident in idents {
                    let mut full = path.to_vec();
                    full.push(ident);
                    if reachable.contains(&full.join("::")) {
                        is_reachable = true;
                        break;
                    }
                }
                if is_reachable {
                    out.push(def);
                }
            }
        }
    }
    out
}

pub(crate) struct RustAxumCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for RustAxumCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
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

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let rest_hir = input_hir.into_rest_hir();
        generate(rest_hir, Path::new(&path), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })
    }
}

fn strip_interfaces(spec: hir::Specification) -> hir::Specification {
    fn strip_defs(defs: Vec<hir::Definition>) -> Vec<hir::Definition> {
        let mut out = Vec::new();
        for def in defs {
            match def {
                hir::Definition::InterfaceDcl(_) => {}
                hir::Definition::ModuleDcl(mut module) => {
                    module.definition = strip_defs(module.definition);
                    if !module.definition.is_empty() {
                        out.push(hir::Definition::ModuleDcl(module));
                    }
                }
                other => out.push(other),
            }
        }
        out
    }

    hir::Specification(strip_defs(spec.0))
}
