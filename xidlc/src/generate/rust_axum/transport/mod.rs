mod convert;
mod model;
mod names;

pub use model::{
    TransportDirection, TransportFieldContext, TransportItemContext, TransportModuleContext,
    TransportModules, TransportTypeDef, TransportVariantContext, TypeRegistry,
};

pub(crate) use convert::{decode_expr, encode_expr};

use super::scope::TypeScope;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{
    array_type, declarator_dims, declarator_name, rust_ident, serde_rename_from_annotations,
};
use names::{canonical_name, transport_ident, transport_module};
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
                    TransportTypeDef::Typedef(def.clone()),
                );
            }
        }
        hir::TypeDcl::NativeDcl(_) => {}
    }
}

fn render_module(
    names: &BTreeSet<String>,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<TransportModuleContext> {
    let mut items = Vec::new();
    for name in names {
        let Some(def) = registry.get(name) else {
            continue;
        };
        match def {
            TransportTypeDef::Struct(def) => items.push(render_struct(
                name,
                def,
                direction,
                module_name,
                registry,
                module_path,
            )?),
            TransportTypeDef::Enum(def) => items.push(render_enum(name, def, module_path)),
            TransportTypeDef::Typedef(_) | TransportTypeDef::Public => {}
        }
    }
    Ok(TransportModuleContext {
        name: module_name.to_string(),
        items,
    })
}

fn render_struct(
    canonical: &str,
    def: &hir::StructDcl,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<TransportItemContext> {
    let mut output = module_path.to_vec();
    output.push(module_name.to_string());
    let mut declaration = canonical
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    declaration.pop();
    let scope = TypeScope {
        registry,
        declaration: &declaration,
        output: &output,
    };
    let mut fields = Vec::new();
    for member in &def.member {
        let rename = serde_rename_from_annotations(&member.annotations);
        for decl in &member.ident {
            let name = rust_ident(&declarator_name(decl));
            let ty = member_ty(member, decl, direction, module_name, scope)?;
            let (enc, dec) = if member.is_optional() {
                let e = encode_expr("value", &member.ty, scope)?;
                let d = decode_expr("value", &member.ty, scope)?;
                let enc = if e == "value" {
                    format!("value.{name}")
                } else {
                    format!("value.{name}.map(|value| {e})")
                };
                let dec = if d == "value" {
                    format!("value.{name}")
                } else {
                    format!("value.{name}.map(|value| {d})")
                };
                (enc, dec)
            } else {
                (
                    encode_expr(&format!("value.{name}"), &member.ty, scope)?,
                    decode_expr(&format!("value.{name}"), &member.ty, scope)?,
                )
            };
            fields.push(TransportFieldContext {
                name: name.clone(),
                ty,
                serde_rename: rename.clone(),
                optional: member.is_optional(),
                encode_expr: enc,
                decode_expr: dec,
            });
        }
    }
    Ok(TransportItemContext {
        kind: "struct".to_string(),
        transport_ident: transport_ident(canonical),
        public_path: TypeScope::relative_path(scope.output, canonical),
        fields,
        variants: Vec::new(),
    })
}

fn render_enum(
    canonical: &str,
    def: &hir::EnumDcl,
    module_path: &[String],
) -> TransportItemContext {
    TransportItemContext {
        kind: "enum".to_string(),
        transport_ident: transport_ident(canonical),
        public_path: format!(
            "super::{}",
            TypeScope::relative_path(module_path, canonical)
        ),
        fields: Vec::new(),
        variants: def
            .member
            .iter()
            .map(|item| TransportVariantContext {
                ident: rust_ident(&item.ident),
                serde_rename: serde_rename_from_annotations(&item.annotations),
            })
            .collect(),
    }
}

fn member_ty(
    member: &hir::Member,
    decl: &hir::Declarator,
    direction: TransportDirection,
    module_name: &str,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    let mut base = map_type_inner(&member.ty, direction, module_name, scope, None)?;
    if member.is_optional() {
        base = format!("Option<{base}>");
    }
    let dims = declarator_dims(decl);
    Ok(if dims.is_empty() {
        base
    } else {
        array_type(&base, &dims)
    })
}

fn map_type_inner(
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
        hir::TypeSpec::SequenceType(seq) => format!(
            "Vec<{}>",
            map_type_inner(&seq.ty, direction, module_name, scope, tracker)?
        ),
        hir::TypeSpec::MapType(map) => {
            let key_ty = map_type_inner(&map.key, direction, module_name, scope, None)?;
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
        Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
            if let Some(tracker) = tracker {
                track_type(key.as_str(), direction, module_name, scope, tracker)?;
            }
            Ok(format!("{module_name}::{}", transport_ident(key.as_str())))
        }
        Some(TransportTypeDef::Typedef(def)) => match &def.ty {
            hir::TypedefType::TypeSpec(ty) => {
                let mut parent = key.split("::").map(str::to_string).collect::<Vec<_>>();
                parent.pop();
                map_type_inner(
                    ty,
                    direction,
                    module_name,
                    scope.in_declaration(&parent),
                    tracker,
                )
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
    if let Some(TransportTypeDef::Struct(def)) = scope.registry.get(name) {
        let mut parent = name.split("::").map(str::to_string).collect::<Vec<_>>();
        parent.pop();
        let scope = scope.in_declaration(&parent);
        for member in &def.member {
            map_type_inner(&member.ty, direction, module_name, scope, Some(tracker))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
