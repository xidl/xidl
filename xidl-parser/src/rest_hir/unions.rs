use super::mapping::is_byte_sequence;
use super::model::{HttpOperation, HttpRepresentation, HttpUnion};
use super::semantics::has_annotation;
use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::HashSet;

impl HttpUnion {
    /// Collects every enum declaration so `@http` unions can resolve their
    /// switch discriminator and derive member media types.
    fn collect_enums(
        definitions: &[hir::Definition],
        module_path: &[String],
        out: &mut Vec<(Vec<String>, hir::EnumDcl)>,
    ) {
        for def in definitions {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    let mut next = module_path.to_vec();
                    next.push(module.ident.clone());
                    Self::collect_enums(&module.definition, &next, out);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                    hir::ConstrTypeDcl::EnumDcl(enum_dcl),
                )) => out.push((module_path.to_vec(), enum_dcl.clone())),
                _ => {}
            }
        }
    }

    /// Collects `@http` unions with their content-negotiated cases resolved.
    pub(super) fn collect(definitions: &[hir::Definition]) -> ParserResult<Vec<HttpUnion>> {
        let mut http_enums = Vec::new();
        Self::collect_enums(definitions, &[], &mut http_enums);
        let mut out = Vec::new();
        fn walk(
            definitions: &[hir::Definition],
            module_path: &[String],
            http_enums: &[(Vec<String>, hir::EnumDcl)],
            out: &mut Vec<HttpUnion>,
        ) -> ParserResult<()> {
            for def in definitions {
                match def {
                    hir::Definition::ModuleDcl(module) => {
                        let mut next = module_path.to_vec();
                        next.push(module.ident.clone());
                        walk(&module.definition, &next, http_enums, out)?;
                    }
                    hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                        hir::ConstrTypeDcl::UnionDef(union),
                    )) => {
                        if has_annotation(&union.annotations, "http") {
                            out.push(HttpUnion {
                                module_path: module_path.to_vec(),
                                ident: union.ident.clone(),
                                cases: HttpUnion::cases(union, module_path, http_enums)?,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        walk(definitions, &[], &http_enums, &mut out)?;
        Ok(out)
    }

    fn cases(
        union: &hir::UnionDef,
        module_path: &[String],
        http_enums: &[(Vec<String>, hir::EnumDcl)],
    ) -> ParserResult<Vec<HttpRepresentation>> {
        let hir::SwitchTypeSpec::ScopedName(scoped) = &union.switch_type_spec else {
            return Err(ParseError::Message(format!(
                "@http union '{}': the switch discriminator must be an enum",
                union.ident
            )));
        };
        let enum_def = Self::resolve_scoped_definition(
            http_enums
                .iter()
                .map(|(path, enum_dcl)| (path.as_slice(), enum_dcl)),
            module_path,
            scoped,
            |enum_dcl| enum_dcl.ident.as_str(),
        )
        .ok_or_else(|| {
            ParseError::Message(format!(
                "@http union '{}': switch enum '{}' does not resolve",
                union.ident,
                scoped.name.join("::")
            ))
        })?;
        let mut cases = Vec::new();
        for case in &union.case {
            if case.label.len() != 1
                || case
                    .label
                    .iter()
                    .any(|label| matches!(label, hir::CaseLabel::Default))
            {
                return Err(ParseError::Message(format!(
                    "@http union '{}': every case needs exactly one enum-member label (no default)",
                    union.ident
                )));
            }
            for label in &case.label {
                let hir::CaseLabel::Value(hir::ConstExpr::ScopedName(label_name)) = label else {
                    return Err(ParseError::Message(format!(
                        "@http union '{}': case labels must name enum members",
                        union.ident
                    )));
                };
                let case_ident = label_name.name.last().cloned().ok_or_else(|| {
                    ParseError::Message(format!("@http union '{}': empty case label", union.ident))
                })?;
                let enumerator = enum_def
                    .member
                    .iter()
                    .find(|member| member.ident == case_ident)
                    .ok_or_else(|| {
                        ParseError::Message(format!(
                            "@http union '{}': case label '{}' is not a member of enum '{}'",
                            union.ident, case_ident, enum_def.ident
                        ))
                    })?;
                let content_type = hir::field_rename(&enumerator.annotations)
                .or_else(|| Self::well_known_media_type(&enumerator.ident))
                .ok_or_else(|| {
                    ParseError::Message(format!(
                        "@http union '{}': cannot derive a media type from enum member '{}'; add @rename(\"...\") to it",
                        union.ident, enumerator.ident
                    ))
                })?;
                let ty = match &case.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => ty.clone(),
                    hir::ElementSpecTy::ConstrTypeDcl(_) => {
                        return Err(ParseError::Message(format!(
                            "@http union '{}': case '{}'-payloads must use named types",
                            union.ident, case_ident
                        )));
                    }
                };
                cases.push(HttpRepresentation {
                    case: case_ident,
                    content_type,
                    is_byte: is_byte_sequence(&ty),
                    ty,
                });
            }
        }
        Ok(cases)
    }

    /// Well-known media types derivable from an enum member name; anything
    /// else needs an explicit `@rename`.
    fn well_known_media_type(ident: &str) -> Option<String> {
        match ident.to_ascii_lowercase().as_str() {
            "json" => Some("application/json".to_string()),
            "octetstream" | "binary" | "bytes" => Some("application/octet-stream".to_string()),
            "text" | "textplain" | "plaintext" => Some("text/plain".to_string()),
            "xml" => Some("application/xml".to_string()),
            "html" => Some("text/html".to_string()),
            "msgpack" => Some("application/msgpack".to_string()),
            "form" | "urlencoded" | "formurlencoded" => {
                Some("application/x-www-form-urlencoded".to_string())
            }
            _ => None,
        }
    }

    /// Resolves a (module path, identifier) pair for a scoped name written
    /// inside `module_path`, walking outward through enclosing modules.
    fn resolve_scoped_definition<'a, T>(
        items: impl Iterator<Item = (&'a [String], &'a T)>,
        module_path: &[String],
        scoped: &hir::ScopedName,
        ident_of: impl Fn(&T) -> &str,
    ) -> Option<&'a T> {
        let ident = scoped.name.last()?;
        let prefix = &scoped.name[..scoped.name.len() - 1];
        let items: Vec<(&'a [String], &'a T)> = items.collect();
        if scoped.is_root {
            return items
                .into_iter()
                .find(|(path, item)| *path == prefix && ident_of(item) == &ident[..])
                .map(|(_, item)| item);
        }
        for skip in (0..=module_path.len()).rev() {
            let scope = &module_path[..skip];
            if scope.ends_with(prefix)
                && let Some(found) = items
                    .iter()
                    .find(|(path, item)| *path == scope && ident_of(item) == &ident[..])
                    .map(|(_, item)| *item)
            {
                return Some(found);
            }
        }
        None
    }
}

impl HttpRepresentation {
    pub(super) fn for_operation(
        operation: &HttpOperation,
        module_path: &[String],
        http_unions: &[HttpUnion],
        has_upgrade: bool,
    ) -> ParserResult<Vec<HttpRepresentation>> {
        let Some(hir::TypeSpec::ScopedName(scoped)) = &operation.signature.return_type else {
            return Ok(Vec::new());
        };
        let items = http_unions.iter().map(|u| (u.module_path.as_slice(), u));
        let union =
            HttpUnion::resolve_scoped_definition(items, module_path, scoped, |u| u.ident.as_str());
        let Some(union) = union else {
            return Ok(Vec::new());
        };
        if union.cases.is_empty() {
            return Ok(Vec::new());
        }
        if operation.meta.stream.kind.is_some() || has_upgrade {
            return Err(ParseError::Message(format!(
                "operation '{}': @http union return types are only supported on non-stream, non-upgrade operations",
                operation.meta.name
            )));
        }
        let mut seen = HashSet::new();
        for case in &union.cases {
            if !seen.insert(case.content_type.clone()) {
                return Err(ParseError::Message(format!(
                    "operation '{}': @http union '{}' declares duplicate media type '{}'",
                    operation.meta.name, union.ident, case.content_type
                )));
            }
        }
        Ok(union.cases.clone())
    }
}
