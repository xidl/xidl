use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{
    array_type, declarator_dims, declarator_name, rust_ident, rust_scoped_name,
    serde_rename_from_annotations,
};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use xidl_parser::hir;

#[derive(Clone)]
pub enum TransportTypeDef {
    Struct(hir::StructDcl),
    Enum(hir::EnumDcl),
    Typedef(hir::TypedefDcl),
}

pub type TypeRegistry = HashMap<String, TransportTypeDef>;

#[derive(Clone, Copy)]
pub enum TransportDirection {
    In,
    Out,
}

#[derive(Serialize)]
pub struct TransportModules {
    pub inbound: TransportModuleContext,
    pub outbound: TransportModuleContext,
}

#[derive(Serialize)]
pub struct TransportModuleContext {
    pub name: String,
    pub items: Vec<TransportItemContext>,
}

#[derive(Serialize)]
pub struct TransportItemContext {
    pub kind: String,
    pub transport_ident: String,
    pub public_path: String,
    pub fields: Vec<TransportFieldContext>,
    pub variants: Vec<String>,
}

#[derive(Serialize)]
pub struct TransportFieldContext {
    pub name: String,
    pub ty: String,
    pub serde_rename: Option<String>,
    pub optional: bool,
    pub encode_expr: String,
    pub decode_expr: String,
}

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

    pub fn map_type(
        &mut self,
        ty: &hir::TypeSpec,
        direction: TransportDirection,
        registry: &TypeRegistry,
    ) -> IdlcResult<String> {
        let module_name = self.module_name(direction).to_string();
        map_type_inner(ty, direction, &module_name, registry, Some(self))
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
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next = module_path.to_vec();
                next.push(module.ident.clone());
                let nested = module.definition.iter().collect::<Vec<_>>();
                collect_registry(&nested, &next, out);
            }
            hir::Definition::TypeDcl(ty) => collect_type_decl(ty, module_path, out),
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
            TransportTypeDef::Typedef(_) => {}
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
    let mut fields = Vec::new();
    for member in &def.member {
        let rename = serde_rename_from_annotations(&member.annotations);
        for decl in &member.ident {
            let name = rust_ident(&declarator_name(decl));
            let ty = member_ty(member, decl, direction, module_name, registry)?;
            #[rustfmt::skip]
            let (enc, dec) = if member.is_optional() {
                let e = encode_expr("value", &member.ty, registry)?;
                let d = decode_expr("value", &member.ty, registry)?;
                (if e == "value" { format!("value.{name}") } else { format!("value.{name}.map(|value| {e})") },
                 if d == "value" { format!("value.{name}") } else { format!("value.{name}.map(|value| {d})") })
            } else {
                (encode_expr(&format!("value.{name}"), &member.ty, registry)?,
                 decode_expr(&format!("value.{name}"), &member.ty, registry)?)
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
        public_path: public_path_from_canonical(canonical, module_path),
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
        public_path: public_path_from_canonical(canonical, module_path),
        fields: Vec::new(),
        variants: def
            .member
            .iter()
            .map(|item| rust_ident(&item.ident))
            .collect(),
    }
}

fn member_ty(
    member: &hir::Member,
    decl: &hir::Declarator,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
) -> IdlcResult<String> {
    let mut base = map_type_inner(&member.ty, direction, module_name, registry, None)?;
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
    registry: &TypeRegistry,
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
            map_type_inner(&seq.ty, direction, module_name, registry, tracker)?
        ),
        hir::TypeSpec::MapType(map) => {
            let key_ty = map_type_inner(&map.key, direction, module_name, registry, None)?;
            let value_ty = map_type_inner(&map.value, direction, module_name, registry, tracker)?;
            format!("::std::collections::BTreeMap<{key_ty}, {value_ty}>")
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(|arg| map_type_inner(arg, direction, module_name, registry, None))
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
        ),
        hir::TypeSpec::ScopedName(value) => {
            map_scoped(value, direction, module_name, registry, tracker)?
        }
    })
}

fn map_scoped(
    value: &hir::ScopedName,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    tracker: Option<&mut TransportTracker>,
) -> IdlcResult<String> {
    let name = scoped_key(value);
    match registry.get(&name) {
        Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
            if let Some(tracker) = tracker {
                track_type(&name, direction, module_name, registry, tracker)?;
            }
            Ok(format!("{module_name}::{}", transport_ident(&name)))
        }
        Some(TransportTypeDef::Typedef(def)) => match &def.ty {
            hir::TypedefType::TypeSpec(ty) => {
                map_type_inner(ty, direction, module_name, registry, tracker)
            }
            hir::TypedefType::ConstrTypeDcl(_) => Err(IdlcError::rpc(format!(
                "unsupported inline typedef transport for '{}'",
                name
            ))),
        },
        None => Ok(render_public_scoped(value)),
    }
}

fn track_type(
    name: &str,
    direction: TransportDirection,
    module_name: &str,
    registry: &TypeRegistry,
    tracker: &mut TransportTracker,
) -> IdlcResult<()> {
    let inserted = match direction {
        TransportDirection::In => tracker.inbound.insert(name.to_string()),
        TransportDirection::Out => tracker.outbound.insert(name.to_string()),
    };
    if !inserted {
        return Ok(());
    }
    if let Some(TransportTypeDef::Struct(def)) = registry.get(name) {
        for member in &def.member {
            map_type_inner(&member.ty, direction, module_name, registry, Some(tracker))?;
        }
    }
    Ok(())
}

#[rustfmt::skip]
pub(crate) fn encode_expr(expr: &str, ty: &hir::TypeSpec, r: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, r)
}

#[rustfmt::skip]
pub(crate) fn decode_expr(expr: &str, ty: &hir::TypeSpec, r: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, r)
}

fn convert_expr(expr: &str, ty: &hir::TypeSpec, registry: &TypeRegistry) -> IdlcResult<String> {
    Ok(match ty {
        hir::TypeSpec::SequenceType(seq) => {
            let inner = convert_expr("value", &seq.ty, registry)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|value| {inner}).collect()")
            }
        }
        hir::TypeSpec::MapType(map) => {
            let inner = convert_expr("value", &map.value, registry)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|(key, value)| (key, {inner})).collect()")
            }
        }
        hir::TypeSpec::ScopedName(value) => {
            let name = scoped_key(value);
            match registry.get(&name) {
                Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
                    format!("{expr}.into()")
                }
                Some(TransportTypeDef::Typedef(def)) => match &def.ty {
                    hir::TypedefType::TypeSpec(inner) => convert_expr(expr, inner, registry)?,
                    hir::TypedefType::ConstrTypeDcl(_) => {
                        return Err(IdlcError::rpc(format!(
                            "unsupported inline typedef transport for '{}'",
                            name
                        )));
                    }
                },
                None => expr.to_string(),
            }
        }
        _ => expr.to_string(),
    })
}

#[rustfmt::skip]
fn canonical_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() { ident.to_string() } else { format!("{}::{}", module_path.join("::"), ident) }
}

#[rustfmt::skip]
fn scoped_key(value: &hir::ScopedName) -> String { value.name.join("::") }

#[rustfmt::skip]
fn transport_ident(value: &str) -> String { value.split("::").map(|part| part.to_string()).collect::<Vec<_>>().join("_") }

#[rustfmt::skip]
fn transport_module(direction: &str, interface_ident: &str) -> String { format!("__xidl_{direction}_{interface_ident}") }

fn public_path_from_canonical(value: &str, module_path: &[String]) -> String {
    let parts = value.split("::").map(rust_ident).collect::<Vec<_>>();
    let current = module_path
        .iter()
        .map(|part| rust_ident(part))
        .collect::<Vec<_>>();
    if parts.starts_with(&current) {
        let suffix = parts[current.len()..].join("::");
        format!("super::{suffix}")
    } else {
        format!("crate::{}", parts.join("::"))
    }
}

#[rustfmt::skip]
fn render_public_scoped(value: &hir::ScopedName) -> String { rust_scoped_name(value) }

#[cfg(test)]
mod tests;
