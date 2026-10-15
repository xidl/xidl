//! Per-version declaration surfaces: what each version module exposes,
//! keyed the way the pairing rules need it.

use std::collections::BTreeMap;

use xidl_parser::hir::{self, effective_wire_name};
use xidl_parser::rest_hir::{HttpOperation, HttpParamKind, RestHirDocument};

/// Everything one version declares that the pairing rules can touch.
#[derive(Debug, Default)]
pub(crate) struct VersionSurface {
    /// Data declarations by type name.
    pub(crate) types: BTreeMap<String, TypeSurface>,
    /// Interfaces by name.
    pub(crate) interfaces: BTreeMap<String, InterfaceSurface>,
}

/// What kind of data declaration a type surface came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeKind {
    /// Struct, exception and union members all pair as members.
    Members,
    /// Enum members pair as values.
    Enum,
}

/// One data declaration and its members.
#[derive(Debug)]
pub(crate) struct TypeSurface {
    pub(crate) kind: TypeKind,
    /// Members or values, in declaration order.
    pub(crate) members: Vec<MemberSurface>,
}

/// One member or value of a data declaration.
#[derive(Debug, Clone)]
pub(crate) struct MemberSurface {
    /// Unprojected declaration path, e.g. `Pet::note` — the interval key.
    pub(crate) decl_path: String,
    /// Language-level identifier.
    pub(crate) ident: String,
    /// The wire name the member travels under.
    pub(crate) wire_name: String,
    /// The declared type; `None` for enum values.
    pub(crate) ty: Option<hir::TypeSpec>,
    pub(crate) optional: bool,
    /// Request location of a parameter; `None` for type members. Two
    /// parameters pair only when their locations match, so moving one
    /// between locations pairs as a replacement.
    pub(crate) location: Option<HttpParamKind>,
}

/// One interface and its operations.
#[derive(Debug)]
pub(crate) struct InterfaceSurface {
    /// Operations by name.
    pub(crate) operations: BTreeMap<String, OperationSurface>,
}

/// One operation: the synthetic request struct its parameters form, plus the
/// outcome side the downcast roots start from.
#[derive(Debug)]
pub(crate) struct OperationSurface {
    /// Parameters in declaration order.
    pub(crate) params: Vec<ParameterSurface>,
    /// The declared return type, when the operation returns one.
    pub(crate) return_type: Option<hir::TypeSpec>,
    /// Exceptions the operation raises, in declaration order.
    pub(crate) raises: Vec<String>,
}

/// One parameter and its request location.
#[derive(Debug)]
pub(crate) struct ParameterSurface {
    /// Unprojected declaration path, e.g. `Pets::get::sku`.
    pub(crate) decl_path: String,
    pub(crate) ident: String,
    /// Request location; a location change pairs as removal plus addition.
    pub(crate) location: HttpParamKind,
    /// The wire name the parameter travels under.
    pub(crate) wire_name: String,
    pub(crate) ty: hir::TypeSpec,
    pub(crate) optional: bool,
}

/// Collects the surface of every version module in the merged specification.
///
/// Top-level modules of a merged projection are the versions; their idents
/// key the surfaces and never join declaration paths, so paths match the
/// unprojected interval paths (`Pet::note`, not `v2::Pet::note`).
pub(crate) fn collect(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
) -> BTreeMap<String, VersionSurface> {
    let mut surfaces: BTreeMap<String, VersionSurface> = BTreeMap::new();
    collect_types(&spec.0, "", "", &mut surfaces);
    for interface in &rest_hir.interfaces {
        let Some(module) = interface.module_path.first() else {
            continue;
        };
        let surface = surfaces.entry(module.clone()).or_default();
        let operations = surface
            .interfaces
            .entry(interface.name.clone())
            .or_insert_with(|| InterfaceSurface {
                operations: BTreeMap::new(),
            });
        for operation in &interface.operations {
            operations.operations.insert(
                operation.meta.name.clone(),
                collect_operation(&interface.name, operation),
            );
        }
    }
    surfaces
}

fn collect_types(
    definitions: &[hir::Definition],
    version: &str,
    path: &str,
    surfaces: &mut BTreeMap<String, VersionSurface>,
) {
    for definition in definitions {
        match definition {
            hir::Definition::ModuleDcl(module_dcl) => {
                if version.is_empty() {
                    collect_types(&module_dcl.definition, &module_dcl.ident, path, surfaces);
                } else {
                    let nested = scoped(path, &module_dcl.ident);
                    collect_types(&module_dcl.definition, version, &nested, surfaces);
                }
            }
            hir::Definition::ConstrTypeDcl(constr) => {
                collect_constr(constr, version, path, surfaces)
            }
            hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(constr)) => {
                collect_constr(constr, version, path, surfaces)
            }
            hir::Definition::ExceptDcl(decl) => {
                push_type(
                    surfaces,
                    version,
                    path,
                    &decl.ident,
                    TypeKind::Members,
                    |type_path| struct_members(&decl.member, type_path, &decl.annotations),
                );
            }
            _ => {}
        }
    }
}

fn collect_constr(
    constr: &hir::ConstrTypeDcl,
    version: &str,
    path: &str,
    surfaces: &mut BTreeMap<String, VersionSurface>,
) {
    match constr {
        hir::ConstrTypeDcl::StructDcl(decl) => {
            push_type(
                surfaces,
                version,
                path,
                &decl.ident,
                TypeKind::Members,
                |type_path| struct_members(&decl.member, type_path, &decl.annotations),
            );
        }
        hir::ConstrTypeDcl::UnionDef(decl) => {
            push_type(
                surfaces,
                version,
                path,
                &decl.ident,
                TypeKind::Members,
                |type_path| union_cases(&decl.case, type_path),
            );
        }
        hir::ConstrTypeDcl::EnumDcl(decl) => {
            push_type(
                surfaces,
                version,
                path,
                &decl.ident,
                TypeKind::Enum,
                |type_path| {
                    decl.member
                        .iter()
                        .map(|enumerator| MemberSurface {
                            decl_path: format!("{type_path}::{}", enumerator.ident),
                            ident: enumerator.ident.clone(),
                            wire_name: effective_wire_name(
                                &enumerator.ident,
                                &enumerator.annotations,
                                &decl.annotations,
                            ),
                            ty: None,
                            optional: false,
                            location: None,
                        })
                        .collect()
                },
            );
        }
        _ => {}
    }
}

fn push_type(
    surfaces: &mut BTreeMap<String, VersionSurface>,
    version: &str,
    path: &str,
    ident: &str,
    kind: TypeKind,
    members: impl FnOnce(&str) -> Vec<MemberSurface>,
) {
    let type_path = scoped(path, ident);
    let surface = surfaces.entry(version.to_string()).or_default();
    surface.types.insert(
        ident.to_string(),
        TypeSurface {
            kind,
            members: members(&type_path),
        },
    );
}

/// Joins a declaration path segment, keeping empty roots empty.
fn scoped(path: &str, ident: &str) -> String {
    if path.is_empty() {
        ident.to_string()
    } else {
        format!("{path}::{ident}")
    }
}

fn struct_members(
    members: &[hir::Member],
    type_path: &str,
    type_annotations: &[hir::Annotation],
) -> Vec<MemberSurface> {
    let mut out = Vec::new();
    for member in members {
        for declarator in &member.ident {
            let raw = declarator_name(declarator);
            out.push(MemberSurface {
                decl_path: format!("{type_path}::{raw}"),
                ident: raw.to_string(),
                wire_name: effective_wire_name(raw, &member.annotations, type_annotations),
                ty: Some(member.ty.clone()),
                optional: member.is_optional(),
                location: None,
            });
        }
    }
    out
}

fn union_cases(cases: &[hir::Case], type_path: &str) -> Vec<MemberSurface> {
    cases
        .iter()
        .map(|case| {
            let ident = declarator_name(&case.element.value).to_string();
            let ty = match &case.element.ty {
                hir::ElementSpecTy::TypeSpec(ty) => Some(ty.clone()),
                hir::ElementSpecTy::ConstrTypeDcl(_) => None,
            };
            MemberSurface {
                decl_path: format!("{type_path}::{ident}"),
                wire_name: ident.clone(),
                ident,
                ty,
                optional: false,
                location: None,
            }
        })
        .collect()
}

fn declarator_name(value: &hir::Declarator) -> &str {
    match value {
        hir::Declarator::SimpleDeclarator(value) => &value.0,
        hir::Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

fn collect_operation(interface_name: &str, operation: &HttpOperation) -> OperationSurface {
    let params = operation
        .signature
        .params
        .iter()
        .filter(|parameter| {
            matches!(
                parameter.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|parameter| {
            let (wire_name, location) = parameter_binding_wire(operation, &parameter.name)
                .unwrap_or((parameter.name.clone(), HttpParamKind::Body));
            ParameterSurface {
                decl_path: format!(
                    "{interface_name}::{}::{}",
                    operation.meta.name, parameter.name
                ),
                ident: parameter.name.clone(),
                location,
                wire_name,
                ty: parameter.ty.clone(),
                optional: parameter.is_optional,
            }
        })
        .collect();
    OperationSurface {
        params,
        return_type: operation.signature.return_type.clone(),
        raises: operation
            .meta
            .raises
            .iter()
            .map(|reference| reference.ident.clone())
            .collect(),
    }
}

/// Resolves a parameter's wire name and request location from the bindings.
fn parameter_binding_wire(
    operation: &HttpOperation,
    parameter: &str,
) -> Option<(String, HttpParamKind)> {
    let locations = [
        (&operation.http.request.path, HttpParamKind::Path),
        (&operation.http.request.query, HttpParamKind::Query),
        (&operation.http.request.header, HttpParamKind::Header),
        (&operation.http.request.cookie, HttpParamKind::Cookie),
    ];
    for (bindings, kind) in locations {
        if let Some(binding) = bindings
            .iter()
            .find(|binding| binding.source_param == parameter)
        {
            return Some((binding.wire_name.clone(), kind));
        }
    }
    None
}
