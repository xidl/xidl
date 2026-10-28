mod convert;
mod member;
mod model;
mod names;
mod render;

pub use model::{
    TransportDirection, TransportFieldContext, TransportItemContext, TransportModuleContext,
    TransportModules, TransportTypeDef, TypeRegistry,
};

pub(crate) use convert::{decode_expr, encode_expr};

use super::scope::TypeScope;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{array_type, declarator_dims, declarator_name, rust_ident};
use names::{canonical_name, transport_ident, transport_module};
use render::render_module;
use std::collections::{BTreeSet, HashMap};
use xidl_parser::hir;

pub struct TransportTracker {
    inbound: BTreeSet<String>,
    outbound: BTreeSet<String>,
    inbound_module: String,
    outbound_module: String,
}

impl TransportTracker {
    pub fn new(interface_ident: &str) -> Self {
        Self {
            inbound: BTreeSet::new(),
            outbound: BTreeSet::new(),
            inbound_module: transport_module("in", interface_ident),
            outbound_module: transport_module("out", interface_ident),
        }
    }

    pub(crate) fn map_type(
        &mut self,
        ty: &hir::TypeSpec,
        direction: TransportDirection,
        scope: TypeScope<'_>,
    ) -> IdlcResult<String> {
        let module_name = self.module_name(direction).to_string();
        map_type_inner(ty, direction, &module_name, scope, Some(self))
    }

    pub fn render_modules(
        &self,
        registry: &TypeRegistry,
        module_path: &[String],
    ) -> IdlcResult<TransportModules> {
        Ok(TransportModules {
            inbound: render_module(
                &self.inbound,
                TransportDirection::In,
                &self.inbound_module,
                registry,
                module_path,
            )?,
            outbound: render_module(
                &self.outbound,
                TransportDirection::Out,
                &self.outbound_module,
                registry,
                module_path,
            )?,
        })
    }

    fn module_name(&self, direction: TransportDirection) -> &str {
        match direction {
            TransportDirection::In => &self.inbound_module,
            TransportDirection::Out => &self.outbound_module,
        }
    }
}

pub fn build_type_registry(defs: &[&hir::Definition], module_path: &[String]) -> TypeRegistry {
    let mut out = HashMap::new();
    collect_registry(defs, module_path, &mut out);
    out
}

fn collect_registry(defs: &[&hir::Definition], module_path: &[String], out: &mut TypeRegistry) {
    for def in defs {
        for ident in super::get_def_idents(def) {
            out.entry(canonical_name(module_path, &ident))
                .or_insert(TransportTypeDef::Public);
        }
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                let nested = module.definition.iter().collect::<Vec<_>>();
                collect_registry(&nested, &next, out);
            }
            hir::Definition::TypeDcl(ty) => collect_type_decl(ty, module_path, out),
            hir::Definition::ExceptDcl(def) => {
                out.insert(
                    canonical_name(module_path, &def.ident),
                    TransportTypeDef::Exception(def.clone()),
                );
            }
            hir::Definition::ConstrTypeDcl(c) => {
                collect_type_decl(&hir::TypeDcl::ConstrTypeDcl(c.clone()), module_path, out)
            }
            _ => {}
        }
    }
}

fn collect_type_decl(ty: &hir::TypeDcl, module_path: &[String], out: &mut TypeRegistry) {
    match ty {
        hir::TypeDcl::ConstrTypeDcl(constr) => match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                out.insert(
                    canonical_name(module_path, &def.ident),
                    TransportTypeDef::Struct(def.clone()),
                );
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                out.insert(
                    canonical_name(module_path, &def.ident),
                    TransportTypeDef::Enum(def.clone()),
                );
            }
            _ => {}
        },
        hir::TypeDcl::TypedefDcl(def) => {
            for decl in &def.decl {
                out.insert(
                    canonical_name(module_path, &declarator_name(decl)),
                    TransportTypeDef::Typedef {
                        ty: def.ty.clone(),
                        declarator: decl.clone(),
                    },
                );
            }
        }
        hir::TypeDcl::NativeDcl(_) => {}
    }
}

pub(super) fn map_type_inner(
    ty: &hir::TypeSpec,
    direction: TransportDirection,
    module_name: &str,
    scope: TypeScope<'_>,
    tracker: Option<&mut TransportTracker>,
) -> IdlcResult<String> {
    Ok(match ty {
        hir::TypeSpec::IntegerType(value) => {
            crate::generate::rust::util::rust_integer_type(value).to_string()
        }
        hir::TypeSpec::FloatingPtType | hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
        hir::TypeSpec::Boolean => "bool".to_string(),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            "::xidl_rust_axum::serde_json::Value".to_string()
        }
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
        hir::TypeSpec::SequenceType(seq) => {
            let element = map_type_inner(&seq.ty, direction, module_name, scope, tracker)?;
            match &seq.len {
                Some(bound) => {
                    let maximum = hir::const_expr_to_i64(&bound.0).ok_or_else(|| {
                        IdlcError::rpc("unresolved sequence bound in Rust transport")
                    })?;
                    super::render::RustAxumRenderer::new()?
                        .render_template(
                            "interface/sequence_type.rs.j2",
                            &serde_json::json!({ "element": element, "maximum": maximum }),
                        )?
                        .trim()
                        .to_owned()
                }
                None => format!("Vec<{element}>"),
            }
        }
        hir::TypeSpec::MapType(map) => {
            let key_ty = scope.public_type(&map.key);
            let value_ty = map_type_inner(&map.value, direction, module_name, scope, tracker)?;
            format!("::std::collections::BTreeMap<{key_ty}, {value_ty}>")
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(|arg| map_type_inner(arg, direction, module_name, scope, None))
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
        ),
        hir::TypeSpec::ScopedName(value) => {
            map_scoped(value, direction, module_name, scope, tracker)?
        }
    })
}

fn map_scoped(
    value: &hir::ScopedName,
    direction: TransportDirection,
    module_name: &str,
    scope: TypeScope<'_>,
    tracker: Option<&mut TransportTracker>,
) -> IdlcResult<String> {
    let canonical = scope.resolve(value);
    let Some(key) = canonical else {
        return Ok(scope.render(value));
    };
    match scope.registry.get(key.as_str()) {
        Some(TransportTypeDef::Struct(_))
        | Some(TransportTypeDef::Exception(_))
        | Some(TransportTypeDef::Enum(_)) => {
            if let Some(tracker) = tracker {
                track_type(key.as_str(), direction, module_name, scope, tracker)?;
            }
            Ok(format!("{module_name}::{}", transport_ident(key.as_str())))
        }
        Some(TransportTypeDef::Typedef { ty, declarator }) => match ty {
            hir::TypedefType::TypeSpec(ty) => {
                let mut parent = key.split("::").map(str::to_string).collect::<Vec<_>>();
                parent.pop();
                let base = map_type_inner(
                    ty,
                    direction,
                    module_name,
                    scope.in_declaration(&parent),
                    tracker,
                )?;
                Ok(array_type(&base, &declarator_dims(declarator)))
            }
            hir::TypedefType::ConstrTypeDcl(_) => Err(IdlcError::rpc(format!(
                "unsupported inline typedef transport for '{key}'"
            ))),
        },
        Some(TransportTypeDef::Public) | None => Ok(scope.render(value)),
    }
}

fn track_type(
    name: &str,
    direction: TransportDirection,
    module_name: &str,
    scope: TypeScope<'_>,
    tracker: &mut TransportTracker,
) -> IdlcResult<()> {
    let inserted = match direction {
        TransportDirection::In => tracker.inbound.insert(name.to_string()),
        TransportDirection::Out => tracker.outbound.insert(name.to_string()),
    };
    if !inserted {
        return Ok(());
    }
    if let Some(def) = scope.registry.get(name).and_then(TransportTypeDef::record) {
        let mut parent = name.split("::").map(str::to_string).collect::<Vec<_>>();
        parent.pop();
        let scope = scope.in_declaration(&parent);
        if let Some(base) = def.parent.first() {
            map_type_inner(
                &hir::TypeSpec::ScopedName(base.clone()),
                direction,
                module_name,
                scope,
                Some(tracker),
            )?;
        }
        for member in &def.member {
            map_type_inner(&member.ty, direction, module_name, scope, Some(tracker))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
