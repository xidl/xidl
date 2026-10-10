mod types;

use super::mapping::is_byte_sequence;
use super::model::{
    HttpOperation, HttpOutputSource, HttpRepresentation, HttpResponseBodyShape, HttpUnion,
};
use super::semantics::has_annotation;
use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::HashSet;
use types::TypeDeclarations;

impl HttpUnion {
    /// Collects `@http` unions with their content-negotiated cases resolved.
    pub(crate) fn collect(definitions: &[hir::Definition]) -> ParserResult<Vec<HttpUnion>> {
        Self::analyze(definitions, false)
    }

    /// Checks user declarations before generated RPC wrappers change their scopes.
    pub(crate) fn validate_source(definitions: &[hir::Definition]) -> ParserResult<()> {
        Self::analyze(definitions, true).map(|_| ())
    }

    fn analyze(
        definitions: &[hir::Definition],
        validate_source: bool,
    ) -> ParserResult<Vec<HttpUnion>> {
        let mut types = TypeDeclarations::collect(definitions);
        if types.http_unions().next().is_none() {
            return Ok(Vec::new());
        }
        let builtins = hir::builtin::HttpBuiltins::load()?;
        types.add_builtins(&builtins)?;
        if validate_source {
            types.validate_runtime_references(definitions, &[])?;
        }
        types
            .http_unions()
            .map(|(module_path, union)| {
                Ok(HttpUnion {
                    module_path: module_path.to_vec(),
                    ident: union.ident.clone(),
                    cases: Self::cases(union, module_path, &types)?,
                })
            })
            .collect()
    }

    fn cases(
        union: &hir::UnionDef,
        module_path: &[String],
        types: &TypeDeclarations<'_>,
    ) -> ParserResult<Vec<HttpRepresentation>> {
        let hir::SwitchTypeSpec::ScopedName(scoped) = &union.switch_type_spec else {
            return Err(ParseError::Message(format!(
                "@http union '{}': the switch discriminator must be the built-in ::ContentType",
                union.ident
            )));
        };
        let enum_def = types.content_type(scoped, module_path).ok_or_else(|| {
            ParseError::Message(format!(
                "@http union '{}': switch '{}' must resolve to the built-in ::ContentType",
                union.ident,
                scoped.name.join("::")
            ))
        })?;
        let mut cases = Vec::new();
        let mut seen = HashSet::new();
        for case in &union.case {
            if has_annotation(&case.element.annotations, "flatten") {
                return Err(ParseError::Message(format!(
                    "@http union '{}': @flatten on cases is unsupported; declare response headers and cookies as operation output parameters",
                    union.ident
                )));
            }
            if matches!(case.element.value, hir::Declarator::ArrayDeclarator(_)) {
                return Err(ParseError::Message(format!(
                    "@http union '{}': array case declarators are unsupported; use a sequence or named struct payload",
                    union.ident
                )));
            }
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
                let enumerator = types
                    .content_type_member(label_name, module_path, enum_def)
                    .ok_or_else(|| {
                        ParseError::Message(format!(
                            "@http union '{}': case label '{}' must name a member of the built-in ::ContentType",
                            union.ident, label_name.name.join("::")
                        ))
                    })?;
                let case_ident = enumerator.ident.clone();
                if !seen.insert(case_ident.clone()) {
                    return Err(ParseError::Message(format!(
                        "@http union '{}': duplicate ContentType case '{}'",
                        union.ident, case_ident
                    )));
                }
                let content_type = hir::field_rename(&enumerator.annotations).ok_or_else(|| {
                    ParseError::Message(format!(
                        "@http union '{}': built-in ContentType member '{}' has no media type",
                        union.ident, enumerator.ident
                    ))
                })?;
                let mut ty = match &case.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => ty.clone(),
                    hir::ElementSpecTy::ConstrTypeDcl(_) => {
                        return Err(ParseError::Message(format!(
                            "@http union '{}': case '{}'-payloads must use named types",
                            union.ident, case_ident
                        )));
                    }
                };
                types.qualify(&mut ty, module_path)?;
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
            let scope = module_path[..skip]
                .iter()
                .chain(prefix)
                .cloned()
                .collect::<Vec<_>>();
            if let Some(found) = items
                .iter()
                .find(|(path, item)| *path == scope.as_slice() && ident_of(item) == &ident[..])
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
        if !matches!(
            operation.http.response.body.shape,
            HttpResponseBodyShape::ReturnOnly { .. }
                | HttpResponseBodyShape::SingleValue {
                    source: HttpOutputSource::ReturnValue,
                    ..
                }
        ) {
            return Err(ParseError::Message(format!(
                "operation '{}': @http union responses cannot have additional body outputs; use header or cookie output parameters",
                operation.meta.name
            )));
        }
        Ok(union.cases.clone())
    }
}
