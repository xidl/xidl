//! Canonical wire surface of a projected specification.
//!
//! A version's surface is the ordered list of facts a wire consumer can
//! observe: struct, exception and union element fields, enum variants and
//! interface operations, each with its effective wire name, canonical type
//! representation and optionality. Surfaces make two versions comparable: the
//! spine joins adjacent versions on `(container, wire name)` and digests pin a
//! released version against silent edits.
//!
//! The canonical wire-name rule is [`hir::effective_wire_name`]; `@skip`
//! removes a declaration from the surface because it never reaches the wire.

use std::collections::HashMap;

use crate::hir::{Definition, Export, Specification, TypeDcl, field_rename, is_skipped};

/// One observable wire fact of a projected version.
#[derive(Debug, Clone)]
pub(crate) struct SurfaceEntry {
    /// Declaration path with the version module stripped, e.g. `Pet::age`.
    pub subject: String,
    /// Container path of the entry, e.g. `Pet`; the correspondence join key.
    pub container: String,
    /// Effective wire name under the canonical rename rule.
    pub wire_name: String,
    /// Canonical representation of the declared type or signature.
    pub type_repr: String,
    /// Canonical HTTP binding of an operation — method, routes, stream or
    /// upgrade protocol — empty for non-HTTP entries. Part of the digest so a
    /// route rename between contract revisions drifts the freeze.
    pub http_repr: String,
    /// Whether the declaration is `@optional`.
    pub optional: bool,
    /// The explicit `@rename` argument, if any; the rename evidence of the
    /// double-signal continuity rule.
    pub explicit_rename: Option<String>,
    /// How the wire carries the entry.
    pub kind: SurfaceKind,
}

/// Kind of surface entry, mirroring how the wire carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfaceKind {
    Field,
    Variant,
    Operation,
}

impl SurfaceKind {
    fn label(self) -> &'static str {
        match self {
            SurfaceKind::Field => "field",
            SurfaceKind::Variant => "variant",
            SurfaceKind::Operation => "operation",
        }
    }
}

impl SurfaceEntry {
    fn label(&self) -> &'static str {
        self.kind.label()
    }
}

pub(super) fn wire_name_of(
    name: &str,
    annotations: &[crate::hir::Annotation],
    container_annotations: &[crate::hir::Annotation],
) -> String {
    crate::hir::effective_wire_name(name, annotations, container_annotations)
}

/// The canonical wire surface of one projected version.
#[derive(Debug, Clone)]
pub(crate) struct VersionSurface {
    entries: Vec<SurfaceEntry>,
}

impl VersionSurface {
    /// Walks a projected specification, stripping the version module root so
    /// paths stay comparable across versions.
    pub(crate) fn of(spec: &Specification, version_module: &str) -> Self {
        let mut entries = Vec::new();
        for definition in &spec.0 {
            match definition {
                Definition::ModuleDcl(module) if module.ident == version_module => {
                    walk_definitions(&module.definition, "", &mut entries);
                }
                other => walk_definition(other, "", &mut entries),
            }
        }
        Self { entries }
    }

    /// Surface entries in declaration order.
    pub(crate) fn entries(&self) -> &[SurfaceEntry] {
        &self.entries
    }

    /// Index of `(container, wire name)` to entry position.
    pub(crate) fn index(&self) -> HashMap<(&str, &str), usize> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| ((entry.container.as_str(), entry.wire_name.as_str()), index))
            .collect()
    }

    /// Canonical digest of the surface; stable across runs and platforms.
    pub(crate) fn digest(&self) -> String {
        let mut canonical = String::new();
        for entry in &self.entries {
            canonical.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                entry.label(),
                entry.container,
                entry.wire_name,
                entry.type_repr,
                entry.http_repr,
                entry.optional
            ));
        }
        format!("md5:{:x}", md5::compute(canonical))
    }
}

fn walk_definitions(definitions: &[Definition], path: &str, entries: &mut Vec<SurfaceEntry>) {
    for definition in definitions {
        walk_definition(definition, path, entries);
    }
}

fn walk_definition(definition: &Definition, path: &str, entries: &mut Vec<SurfaceEntry>) {
    match definition {
        Definition::ModuleDcl(module) => {
            let here = super::paths::join(path, &module.ident);
            walk_definitions(&module.definition, &here, entries);
        }
        Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
            walk_constr_type(constr, path, entries);
        }
        Definition::ExceptDcl(except) => {
            let here = super::paths::join(path, &except.ident);
            walk_fields(&except.member, &here, &except.annotations, entries);
        }
        Definition::InterfaceDcl(interface) => {
            if let crate::hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl {
                let here = super::paths::join(path, &def.header.ident);
                if let Some(body) = &def.interface_body {
                    for export in &body.0 {
                        if let Export::OpDcl(op) = export {
                            entries.push(SurfaceEntry {
                                subject: super::paths::join(&here, &op.ident),
                                container: here.clone(),
                                wire_name: op.ident.clone(),
                                type_repr: operation_repr(op),
                                http_repr: crate::rest_hir::canonical_http_binding(&op.annotations)
                                    .unwrap_or_default(),
                                optional: false,
                                explicit_rename: field_rename(&op.annotations),
                                kind: SurfaceKind::Operation,
                            });
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn walk_constr_type(
    constr: &crate::hir::ConstrTypeDcl,
    path: &str,
    entries: &mut Vec<SurfaceEntry>,
) {
    match constr {
        crate::hir::ConstrTypeDcl::StructDcl(def) => {
            let here = super::paths::join(path, &def.ident);
            walk_fields(&def.member, &here, &def.annotations, entries);
        }
        crate::hir::ConstrTypeDcl::EnumDcl(def) => {
            let here = super::paths::join(path, &def.ident);
            for member in &def.member {
                entries.push(SurfaceEntry {
                    subject: super::paths::join(&here, &member.ident),
                    container: here.clone(),
                    wire_name: wire_name_of(&member.ident, &member.annotations, &def.annotations),
                    type_repr: "enum".to_string(),
                    http_repr: String::new(),
                    optional: false,
                    explicit_rename: field_rename(&member.annotations),
                    kind: SurfaceKind::Variant,
                });
            }
        }
        crate::hir::ConstrTypeDcl::UnionDef(def) => {
            let here = super::paths::join(path, &def.ident);
            for case in &def.case {
                let Some(name) = declarator_ident(&case.element.value) else {
                    continue;
                };
                entries.push(SurfaceEntry {
                    subject: super::paths::join(&here, &name),
                    container: here.clone(),
                    wire_name: wire_name_of(&name, &case.element.annotations, &def.annotations),
                    type_repr: element_type_repr(&case.element),
                    http_repr: String::new(),
                    optional: false,
                    explicit_rename: field_rename(&case.element.annotations),
                    kind: SurfaceKind::Field,
                });
            }
        }
        _ => {}
    }
}

fn walk_fields(
    members: &[crate::hir::Member],
    container: &str,
    container_annotations: &[crate::hir::Annotation],
    entries: &mut Vec<SurfaceEntry>,
) {
    for member in members {
        if is_skipped(&member.annotations) {
            continue;
        }
        for decl in &member.ident {
            let Some(name) = declarator_ident(decl) else {
                continue;
            };
            let mut repr = type_repr(&member.ty);
            if let crate::hir::Declarator::ArrayDeclarator(array) = decl {
                for _ in &array.len {
                    repr = format!("sequence<{repr}>");
                }
            }
            entries.push(SurfaceEntry {
                subject: super::paths::join(container, &name),
                container: container.to_string(),
                wire_name: wire_name_of(&name, &member.annotations, container_annotations),
                type_repr: repr,
                http_repr: String::new(),
                optional: member.is_optional(),
                explicit_rename: field_rename(&member.annotations),
                kind: SurfaceKind::Field,
            });
        }
    }
}

pub(super) fn declarator_ident(decl: &crate::hir::Declarator) -> Option<String> {
    match decl {
        crate::hir::Declarator::SimpleDeclarator(simple) => Some(simple.0.clone()),
        crate::hir::Declarator::ArrayDeclarator(array) => Some(array.ident.clone()),
    }
}

fn element_type_repr(element: &crate::hir::ElementSpec) -> String {
    match &element.ty {
        crate::hir::ElementSpecTy::TypeSpec(ty) => type_repr(ty),
        crate::hir::ElementSpecTy::ConstrTypeDcl(_) => "inline".to_string(),
    }
}

fn operation_repr(op: &crate::hir::OpDcl) -> String {
    let ret = match &op.ty {
        crate::hir::OpTypeSpec::Void => "void".to_string(),
        crate::hir::OpTypeSpec::TypeSpec(ty) => type_repr(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|parameter| {
            parameter
                .0
                .iter()
                .map(|param| format!("{}:{}", param.declarator.0, type_repr(&param.ty)))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let raises = op.raises.as_ref().map(|raises| {
        raises
            .0
            .iter()
            .map(render_scoped_name)
            .collect::<Vec<_>>()
            .join(", ")
    });
    match raises {
        Some(raises) => format!("{ret}({params})raises({raises})"),
        None => format!("{ret}({params})"),
    }
}

/// Canonical representation of a type; structural, not source spelling.
pub(crate) fn type_repr(ty: &crate::hir::TypeSpec) -> String {
    match ty {
        crate::hir::TypeSpec::IntegerType(integer) => integer_repr(integer).to_string(),
        crate::hir::TypeSpec::FloatingPtType => "floating".to_string(),
        crate::hir::TypeSpec::CharType => "char".to_string(),
        crate::hir::TypeSpec::WideCharType => "wchar".to_string(),
        crate::hir::TypeSpec::Boolean => "boolean".to_string(),
        crate::hir::TypeSpec::AnyType => "any".to_string(),
        crate::hir::TypeSpec::ObjectType => "Object".to_string(),
        crate::hir::TypeSpec::ValueBaseType => "ValueBase".to_string(),
        crate::hir::TypeSpec::ScopedName(name) => render_scoped_name(name),
        crate::hir::TypeSpec::SequenceType(sequence) => match sequence.len {
            Some(_) => format!("sequence<{};bound>", type_repr(&sequence.ty)),
            None => format!("sequence<{}>", type_repr(&sequence.ty)),
        },
        crate::hir::TypeSpec::StringType(bound) => bound_repr("string", bound.bound.as_ref()),
        crate::hir::TypeSpec::WideStringType(bound) => bound_repr("wstring", bound.bound.as_ref()),
        crate::hir::TypeSpec::FixedPtType(_) => "fixed".to_string(),
        crate::hir::TypeSpec::MapType(map) => {
            format!("map<{}, {}>", type_repr(&map.key), type_repr(&map.value))
        }
        crate::hir::TypeSpec::TemplateType(template) => {
            let args = template
                .args
                .iter()
                .map(type_repr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{args}>", template.ident)
        }
    }
}

fn integer_repr(integer: &crate::hir::IntegerType) -> &'static str {
    match integer {
        crate::hir::IntegerType::Char => "char",
        crate::hir::IntegerType::UChar => "uchar",
        crate::hir::IntegerType::Octet => "octet",
        crate::hir::IntegerType::U8 => "u8",
        crate::hir::IntegerType::U16 => "u16",
        crate::hir::IntegerType::U32 => "u32",
        crate::hir::IntegerType::U64 => "u64",
        crate::hir::IntegerType::I8 => "i8",
        crate::hir::IntegerType::I16 => "i16",
        crate::hir::IntegerType::I32 => "i32",
        crate::hir::IntegerType::I64 => "i64",
    }
}

fn bound_repr(name: &str, bound: Option<&crate::hir::PositiveIntConst>) -> String {
    match bound {
        Some(_) => format!("{name}<bound>"),
        None => name.to_string(),
    }
}

fn render_scoped_name(name: &crate::hir::ScopedName) -> String {
    let prefix = if name.is_root { "::" } else { "" };
    format!("{prefix}{}", name.name.join("::"))
}
