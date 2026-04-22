use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::{
    array_type, declarator_dims, declarator_name, rust_ident, rust_scoped_name,
    serde_rename_from_annotations,
};
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

pub struct TransportModules {
    pub inbound: String,
    pub outbound: String,
}

#[derive(Default)]
pub struct TransportTracker {
    inbound: BTreeSet<String>,
    outbound: BTreeSet<String>,
}

impl TransportTracker {
    pub fn map_type(
        &mut self,
        ty: &hir::TypeSpec,
        direction: TransportDirection,
        registry: &TypeRegistry,
    ) -> IdlcResult<String> {
        map_type_inner(ty, direction, registry, Some(self))
    }

    pub fn render_modules(
        &self,
        registry: &TypeRegistry,
        module_path: &[String],
    ) -> IdlcResult<TransportModules> {
        Ok(TransportModules {
            inbound: render_module(&self.inbound, TransportDirection::In, registry, module_path)?,
            outbound: render_module(
                &self.outbound,
                TransportDirection::Out,
                registry,
                module_path,
            )?,
        })
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
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<String> {
    let mut out = String::new();
    for name in names {
        let Some(def) = registry.get(name) else {
            continue;
        };
        match def {
            TransportTypeDef::Struct(def) => {
                render_struct(&mut out, name, def, direction, registry, module_path)?
            }
            TransportTypeDef::Enum(def) => render_enum(&mut out, name, def, module_path)?,
            TransportTypeDef::Typedef(_) => {}
        }
    }
    Ok(out)
}

fn render_struct(
    out: &mut String,
    canonical: &str,
    def: &hir::StructDcl,
    direction: TransportDirection,
    registry: &TypeRegistry,
    module_path: &[String],
) -> IdlcResult<()> {
    let transport = transport_ident(canonical);
    let public = public_path_from_canonical(canonical, module_path);
    out.push_str("#[derive(::serde::Serialize, ::serde::Deserialize)]\n");
    out.push_str(&format!("pub(super) struct {transport} {{\n"));
    for member in &def.member {
        let rename = serde_rename_from_annotations(&member.annotations);
        for decl in &member.ident {
            if let Some(rename) = &rename {
                out.push_str(&format!("    #[serde(rename = \"{rename}\")]\n"));
            }
            if member.is_optional() {
                out.push_str("    #[serde(default)]\n");
            }
            let name = rust_ident(&declarator_name(decl));
            let ty = member_ty(member, decl, direction, registry)?;
            out.push_str(&format!("    {name}: {ty},\n"));
        }
    }
    out.push_str("}\n\n");
    out.push_str(&format!("impl From<{public}> for {transport} {{\n"));
    out.push_str(&format!("    fn from(value: {public}) -> Self {{\n"));
    out.push_str("        Self {\n");
    for member in &def.member {
        for decl in &member.ident {
            let name = rust_ident(&declarator_name(decl));
            out.push_str(&format!(
                "            {name}: {} ,\n",
                encode_expr(&format!("value.{name}"), &member.ty, registry)?
            ));
        }
    }
    out.push_str("        }\n    }\n}\n\n");
    out.push_str(&format!("impl From<{transport}> for {public} {{\n"));
    out.push_str(&format!("    fn from(value: {transport}) -> Self {{\n"));
    out.push_str("        Self {\n");
    for member in &def.member {
        for decl in &member.ident {
            let name = rust_ident(&declarator_name(decl));
            out.push_str(&format!(
                "            {name}: {} ,\n",
                decode_expr(&format!("value.{name}"), &member.ty, registry)?
            ));
        }
    }
    out.push_str("        }\n    }\n}\n\n");
    Ok(())
}

fn render_enum(
    out: &mut String,
    canonical: &str,
    def: &hir::EnumDcl,
    module_path: &[String],
) -> IdlcResult<()> {
    let transport = transport_ident(canonical);
    let public = public_path_from_canonical(canonical, module_path);
    out.push_str("#[derive(::serde::Serialize, ::serde::Deserialize)]\n");
    out.push_str(&format!("pub(super) enum {transport} {{\n"));
    for item in &def.member {
        out.push_str(&format!("    {},\n", rust_ident(&item.ident)));
    }
    out.push_str("}\n\n");
    out.push_str(&format!("impl From<{public}> for {transport} {{\n"));
    out.push_str(&format!(
        "    fn from(value: {public}) -> Self {{\n        match value {{\n"
    ));
    for item in &def.member {
        let name = rust_ident(&item.ident);
        out.push_str(&format!("            {public}::{name} => Self::{name},\n"));
    }
    out.push_str("        }\n    }\n}\n\n");
    out.push_str(&format!("impl From<{transport}> for {public} {{\n"));
    out.push_str(&format!(
        "    fn from(value: {transport}) -> Self {{\n        match value {{\n"
    ));
    for item in &def.member {
        let name = rust_ident(&item.ident);
        out.push_str(&format!(
            "            {transport}::{name} => Self::{name},\n"
        ));
    }
    out.push_str("        }\n    }\n}\n\n");
    Ok(())
}

fn member_ty(
    member: &hir::Member,
    decl: &hir::Declarator,
    direction: TransportDirection,
    registry: &TypeRegistry,
) -> IdlcResult<String> {
    let mut base = map_type_inner(&member.ty, direction, registry, None)?;
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
            map_type_inner(&seq.ty, direction, registry, tracker)?
        ),
        hir::TypeSpec::MapType(map) => {
            let key_ty = map_type_inner(&map.key, direction, registry, None)?;
            let value_ty = map_type_inner(&map.value, direction, registry, tracker)?;
            format!("::std::collections::BTreeMap<{key_ty}, {value_ty}>")
        }
        hir::TypeSpec::TemplateType(value) => format!(
            "{}<{}>",
            rust_ident(&value.ident),
            value
                .args
                .iter()
                .map(|arg| map_type_inner(arg, direction, registry, None))
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
        ),
        hir::TypeSpec::ScopedName(value) => map_scoped(value, direction, registry, tracker)?,
    })
}

fn map_scoped(
    value: &hir::ScopedName,
    direction: TransportDirection,
    registry: &TypeRegistry,
    tracker: Option<&mut TransportTracker>,
) -> IdlcResult<String> {
    let name = scoped_key(value);
    match registry.get(&name) {
        Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
            if let Some(tracker) = tracker {
                track_type(&name, direction, registry, tracker)?;
            }
            let module = match direction {
                TransportDirection::In => "__xidl_in",
                TransportDirection::Out => "__xidl_out",
            };
            Ok(format!("{module}::{}", transport_ident(&name)))
        }
        Some(TransportTypeDef::Typedef(def)) => match &def.ty {
            hir::TypedefType::TypeSpec(ty) => map_type_inner(ty, direction, registry, tracker),
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
            map_type_inner(&member.ty, direction, registry, Some(tracker))?;
        }
    }
    Ok(())
}

fn encode_expr(expr: &str, ty: &hir::TypeSpec, registry: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, registry)
}

fn decode_expr(expr: &str, ty: &hir::TypeSpec, registry: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, registry)
}

fn convert_expr(expr: &str, ty: &hir::TypeSpec, registry: &TypeRegistry) -> IdlcResult<String> {
    Ok(match ty {
        hir::TypeSpec::SequenceType(seq) => format!(
            "{expr}.into_iter().map(|value| {}).collect()",
            convert_expr("value", &seq.ty, registry)?
        ),
        hir::TypeSpec::MapType(map) => format!(
            "{expr}.into_iter().map(|(key, value)| (key, {})).collect()",
            convert_expr("value", &map.value, registry)?
        ),
        hir::TypeSpec::ScopedName(value) => {
            let name = scoped_key(value);
            match registry.get(&name) {
                Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
                    ".into()".replacen(".", expr, 1)
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

fn canonical_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        format!("{}::{}", module_path.join("::"), ident)
    }
}

fn scoped_key(value: &hir::ScopedName) -> String {
    value.name.join("::")
}

fn transport_ident(value: &str) -> String {
    value
        .split("::")
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join("_")
}

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

fn render_public_scoped(value: &hir::ScopedName) -> String {
    rust_scoped_name(value)
}
