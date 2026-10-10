use super::model::{HttpException, HttpExceptionField, HttpExceptionMember, HttpExceptionRef};
use super::semantics::{has_annotation, normalize_annotation_params};
use crate::error::{ParseError, ParserResult};
use crate::hir;
use std::collections::HashSet;

impl HttpException {
    pub(super) fn from_hir(except: &hir::ExceptDcl, module_path: &[String]) -> ParserResult<Self> {
        let status = Self::status(&except.annotations, &except.ident)?;
        let mut headers = Vec::new();
        let mut cookies = Vec::new();
        let mut body = Vec::new();
        for member in &except.member {
            for decl in &member.ident {
                let field = match decl {
                    hir::Declarator::SimpleDeclarator(value) => value.0.clone(),
                    hir::Declarator::ArrayDeclarator(value) => {
                        return Err(ParseError::Message(format!(
                            "exception '{}': array declarators are not supported ('{}')",
                            except.ident, value.ident
                        )));
                    }
                };
                let wire_name =
                    hir::effective_wire_name(&field, &member.annotations, &except.annotations);
                if has_annotation(&member.annotations, "header") {
                    headers.push(HttpExceptionMember {
                        field,
                        wire_name,
                        ty: member.ty.clone(),
                        is_multi: matches!(member.ty, hir::TypeSpec::SequenceType(_)),
                        optional: member.is_optional(),
                    });
                } else if has_annotation(&member.annotations, "cookie") {
                    cookies.push(HttpExceptionMember {
                        field,
                        wire_name,
                        ty: member.ty.clone(),
                        is_multi: matches!(member.ty, hir::TypeSpec::SequenceType(_)),
                        optional: member.is_optional(),
                    });
                } else {
                    body.push(HttpExceptionField {
                        field,
                        wire_name,
                        optional: member.is_optional(),
                        ty: member.ty.clone(),
                    });
                }
            }
        }
        if status == 304 && !body.is_empty() {
            return Err(ParseError::Message(format!(
                "exception '{}': HTTP 304 cannot carry body members",
                except.ident
            )));
        }
        Ok(Self {
            module_path: module_path.to_vec(),
            ident: except.ident.clone(),
            status,
            headers,
            cookies,
            body,
        })
    }

    fn http_params(annotations: &[hir::Annotation]) -> Option<&hir::AnnotationParams> {
        annotations.iter().find_map(|annotation| match annotation {
            hir::Annotation::Builtin { name, params } if name == "http" => params.as_ref(),
            hir::Annotation::ScopedName { name, params }
                if name.name.last().map(String::as_str) == Some("http") =>
            {
                params.as_ref()
            }
            _ => None,
        })
    }

    fn status(annotations: &[hir::Annotation], ident: &str) -> ParserResult<u16> {
        const HINT: &str = "(e.g. @http(412) or @http(status = 412))";
        let params = Self::http_params(annotations).ok_or_else(|| {
            ParseError::Message(format!(
                "exception '{ident}': @http <status> is required for the HTTP error channel {HINT}"
            ))
        })?;
        let params = normalize_annotation_params(params);
        let raw = params
            .get("status")
            .or_else(|| params.get("value"))
            .ok_or_else(|| {
                ParseError::Message(format!(
                    "exception '{ident}': @http requires an integer status {HINT}"
                ))
            })?;
        let status: u16 = raw.trim().parse().map_err(|_| {
            ParseError::Message(format!(
                "exception '{ident}': @http status '{raw}' is not an integer {HINT}"
            ))
        })?;
        if !(300..=599).contains(&status) {
            return Err(ParseError::Message(format!(
                "exception '{ident}': @http status {status} must be in 300..=599; success responses belong to the return type"
            )));
        }
        Ok(status)
    }

    fn resolve<'a>(
        exceptions: &'a [HttpException],
        module_path: &[String],
        scoped: &hir::ScopedName,
    ) -> Option<&'a HttpException> {
        let ident = scoped.name.last()?;
        let prefix = &scoped.name[..scoped.name.len() - 1];
        if scoped.is_root {
            return exceptions
                .iter()
                .find(|e| &e.ident == ident && e.module_path.as_slice() == prefix);
        }
        let depth = module_path.len();
        for skip in (0..=depth).rev() {
            let scope = &module_path[..skip];
            if let Some(found) = exceptions
                .iter()
                .find(|e| &e.ident == ident && e.module_path.iter().eq(scope.iter().chain(prefix)))
            {
                return Some(found);
            }
        }
        None
    }
}

impl HttpExceptionRef {
    pub(super) fn for_operation(
        op: &hir::OpDcl,
        module_path: &[String],
        exceptions: &[HttpException],
    ) -> ParserResult<Vec<HttpExceptionRef>> {
        let Some(expr) = &op.raises else {
            return Ok(Vec::new());
        };
        let op_ident = &op.ident;
        let mut refs = Vec::new();
        let mut statuses = HashSet::new();
        for scoped in &expr.0 {
            let exception = HttpException::resolve(exceptions, module_path, scoped).ok_or_else(|| {
            ParseError::Message(format!(
                "operation '{op_ident}': raises('{}') does not resolve to a declared exception",
                scoped.name.join("::")
            ))
        })?;
            if !statuses.insert(exception.status) {
                return Err(ParseError::Message(format!(
                    "operation '{op_ident}': duplicate HTTP status {} in raises(...)",
                    exception.status
                )));
            }
            refs.push(HttpExceptionRef {
                module_path: exception.module_path.clone(),
                ident: exception.ident.clone(),
            });
        }
        Ok(refs)
    }
}
