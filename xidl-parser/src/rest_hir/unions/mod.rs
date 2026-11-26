use super::mapping::is_byte_sequence;
use super::model::{
    HttpOperation, HttpOutputSource, HttpRepresentation, HttpResponseBodyShape, HttpUnion,
};
use super::semantics::has_annotation;
use super::types::TypeDeclarations;
use crate::diagnostic::{Diagnostic, DiagnosticBag, DiagnosticCode};
use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::HashSet;

impl HttpUnion {
    /// Collects `@http` unions with their content-negotiated cases resolved.
    pub(crate) fn collect(definitions: &[hir::Definition]) -> ParserResult<Vec<HttpUnion>> {
        let mut bag = DiagnosticBag::new();
        let unions = Self::analyze_into(definitions, false, &mut bag);
        bag.check_errors()?;
        Ok(unions)
    }

    /// Collects all `@http` union validation errors into `bag`.
    pub(crate) fn collect_diagnostics(
        definitions: &[hir::Definition],
        validate_source: bool,
        bag: &mut DiagnosticBag,
    ) {
        let _ = Self::analyze_into(definitions, validate_source, bag);
    }

    pub(crate) fn analyze_into(
        definitions: &[hir::Definition],
        validate_source: bool,
        bag: &mut DiagnosticBag,
    ) -> Vec<HttpUnion> {
        let mut types = TypeDeclarations::collect(definitions);
        if types.http_unions().next().is_none() {
            return Vec::new();
        }
        let builtins = match hir::builtin::HttpBuiltins::load() {
            Ok(builtins) => builtins,
            Err(err) => {
                bag.push(Diagnostic::error(
                    DiagnosticCode::ProtocolError,
                    0..0,
                    err.to_string(),
                ));
                return Vec::new();
            }
        };
        if let Err(err) = types.add_builtins(&builtins) {
            bag.push(Diagnostic::error(
                DiagnosticCode::ProtocolError,
                0..0,
                err.to_string(),
            ));
            return Vec::new();
        }
        if validate_source && let Err(err) = types.validate_runtime_references(definitions, &[]) {
            bag.push(Diagnostic::error(
                DiagnosticCode::ProtocolError,
                0..0,
                err.to_string(),
            ));
        }
        types
            .http_unions()
            .map(|(module_path, union)| HttpUnion {
                module_path: module_path.to_vec(),
                ident: union.ident.clone(),
                cases: Self::cases_into(union, module_path, &types, bag),
            })
            .collect()
    }

    fn cases_into(
        union: &hir::UnionDef,
        module_path: &[String],
        types: &TypeDeclarations<'_>,
        bag: &mut DiagnosticBag,
    ) -> Vec<HttpRepresentation> {
        let location = Self::union_location(module_path, &union.ident);
        let hir::SwitchTypeSpec::ScopedName(scoped) = &union.switch_type_spec else {
            Self::push_error(
                bag,
                &location,
                format!(
                    "@http union '{}': the switch discriminator must be the built-in ::ContentType",
                    union.ident
                ),
            );
            return Vec::new();
        };
        let Some(enum_def) = types.content_type(scoped, module_path) else {
            Self::push_error(
                bag,
                &location,
                format!(
                    "@http union '{}': switch '{}' must resolve to the built-in ::ContentType",
                    union.ident,
                    scoped.name.join("::")
                ),
            );
            return Vec::new();
        };
        let mut cases = Vec::new();
        let mut seen = HashSet::new();
        for case in &union.case {
            if has_annotation(&case.element.annotations, "flatten") {
                Self::push_error(
                    bag,
                    &location,
                    format!(
                        "@http union '{}': @flatten on cases is unsupported; declare response headers and cookies as operation output parameters",
                        union.ident
                    ),
                );
                continue;
            }
            if matches!(case.element.value, hir::Declarator::ArrayDeclarator(_)) {
                Self::push_error(
                    bag,
                    &location,
                    format!(
                        "@http union '{}': array case declarators are unsupported; use a sequence or named struct payload",
                        union.ident
                    ),
                );
                continue;
            }
            if case.label.len() != 1
                || case
                    .label
                    .iter()
                    .any(|label| matches!(label, hir::CaseLabel::Default))
            {
                Self::push_error(
                    bag,
                    &location,
                    format!(
                        "@http union '{}': every case needs exactly one enum-member label (no default)",
                        union.ident
                    ),
                );
                continue;
            }
            for label in &case.label {
                let hir::CaseLabel::Value(hir::ConstExpr::ScopedName(label_name)) = label else {
                    Self::push_error(
                        bag,
                        &location,
                        format!(
                            "@http union '{}': case labels must name enum members",
                            union.ident
                        ),
                    );
                    continue;
                };
                let Some(enumerator) = types.content_type_member(label_name, module_path, enum_def)
                else {
                    Self::push_error(
                        bag,
                        &location,
                        format!(
                            "@http union '{}': case label '{}' must name a member of the built-in ::ContentType",
                            union.ident,
                            label_name.name.join("::")
                        ),
                    );
                    continue;
                };
                let case_ident = enumerator.ident.clone();
                if !seen.insert(case_ident.clone()) {
                    Self::push_error(
                        bag,
                        &location,
                        format!(
                            "@http union '{}': duplicate ContentType case '{}'",
                            union.ident, case_ident
                        ),
                    );
                    continue;
                }
                let Some(content_type) = xidl_http::ContentType::from_idl_name(&enumerator.ident)
                else {
                    Self::push_error(
                        bag,
                        &location,
                        format!(
                            "@http union '{}': built-in ContentType member '{}' has no runtime definition",
                            union.ident, enumerator.ident
                        ),
                    );
                    continue;
                };
                let mut ty = match &case.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => ty.clone(),
                    hir::ElementSpecTy::ConstrTypeDcl(_) => {
                        Self::push_error(
                            bag,
                            &location,
                            format!(
                                "@http union '{}': case '{}'-payloads must use named types",
                                union.ident, case_ident
                            ),
                        );
                        continue;
                    }
                };
                if let Err(err) = types.qualify(&mut ty, module_path) {
                    Self::push_error(bag, &location, err.to_string());
                    continue;
                }
                cases.push(HttpRepresentation {
                    content_type,
                    is_byte: is_byte_sequence(&ty),
                    ty,
                });
            }
        }
        cases
    }

    fn union_location(module_path: &[String], ident: &str) -> String {
        module_path
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(ident))
            .collect::<Vec<_>>()
            .join("::")
    }

    fn push_error(bag: &mut DiagnosticBag, location: &str, message: String) {
        bag.push(
            Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, message)
                .with_location(location.to_string()),
        );
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
        let union = TypeDeclarations::resolve_scoped_definition(items, module_path, scoped, |u| {
            u.ident.as_str()
        });
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
