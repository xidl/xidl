//! Explicit HTTP sources and their effective wire names.

use crate::hir;
use crate::lint::AnnotationKind;
use crate::rest_hir::{HttpParamKind, RestHirResult};

/// One explicit HTTP binding shared by operation parameters and exceptions.
#[derive(Clone)]
pub(crate) struct SourceBinding {
    pub(crate) source: HttpParamKind,
    name: Option<String>,
}

impl SourceBinding {
    pub(crate) fn read(
        annotations: &[hir::Annotation],
        owner: &str,
    ) -> RestHirResult<Option<Self>> {
        let mut binding: Option<Self> = None;
        for annotation in annotations {
            let source = match hir::annotation_name(annotation).and_then(AnnotationKind::from_name)
            {
                Some(AnnotationKind::Path) => HttpParamKind::Path,
                Some(AnnotationKind::Query) => HttpParamKind::Query,
                Some(AnnotationKind::Header) => HttpParamKind::Header,
                Some(AnnotationKind::Cookie) => HttpParamKind::Cookie,
                Some(AnnotationKind::Body) => HttpParamKind::Body,
                _ => continue,
            };
            let name = Self::annotation_wire_name(annotation, owner, source)?;
            match &mut binding {
                None => binding = Some(Self { source, name }),
                Some(previous) if previous.source != source => {
                    return Err(format!(
                        "HTTP binding '{owner}' has conflicting source annotations (@path/@query/@body/@header/@cookie)"
                    ));
                }
                Some(previous) => {
                    if let Some(name) = name {
                        if previous.name.as_ref().is_some_and(|value| {
                            if source == HttpParamKind::Header {
                                !value.eq_ignore_ascii_case(&name)
                            } else {
                                value != &name
                            }
                        }) {
                            return Err(format!(
                                "HTTP binding '{owner}' declares conflicting wire names for the same source"
                            ));
                        }
                        previous.name.get_or_insert(name);
                    }
                }
            }
        }
        Ok(binding)
    }

    fn annotation_wire_name(
        annotation: &hir::Annotation,
        owner: &str,
        source: HttpParamKind,
    ) -> RestHirResult<Option<String>> {
        let Some(params) = hir::annotation_params(annotation) else {
            return Ok(None);
        };
        let valid_string = match params {
            hir::AnnotationParams::ConstExpr(expression) => {
                matches!(
                    expression,
                    hir::ConstExpr::Literal(hir::Literal::StringLiteral(_))
                )
            }
            hir::AnnotationParams::Params(values) => {
                values.len() == 1
                    && values[0].value.as_ref().is_some_and(|expression| {
                        matches!(
                            expression,
                            hir::ConstExpr::Literal(hir::Literal::StringLiteral(_))
                        )
                    })
            }
            hir::AnnotationParams::Positional(_) => false,
            hir::AnnotationParams::Raw(raw) => {
                let raw = crate::parser::raw::RawArguments::without_comments(raw);
                if raw.trim().is_empty() {
                    return Ok(None);
                }
                let parts = crate::parser::raw::RawArguments::split(&raw, ',');
                parts.len() <= 1
                    && parts.first().is_none_or(|part| {
                        let pair = crate::parser::raw::RawArguments::split(part, '=');
                        let value = match pair.as_slice() {
                            [value] | [_, value] => value.trim(),
                            _ => return false,
                        };
                        value.starts_with('"') && value.ends_with('"')
                    })
            }
        };
        let normalized_params = match params {
            hir::AnnotationParams::Raw(raw) => {
                let raw = crate::parser::raw::RawArguments::without_comments(raw);
                let parts = crate::parser::raw::RawArguments::split(&raw, '=');
                let (key, value) = match parts.as_slice() {
                    [value] => ("value", *value),
                    [key, value] => (key.trim(), *value),
                    _ => {
                        return Err(format!(
                            "HTTP binding '{owner}' has invalid source arguments"
                        ));
                    }
                };
                hir::AnnotationParams::Params(vec![hir::AnnotationParam {
                    ident: key.to_owned(),
                    value: Some(hir::ConstExpr::Literal(hir::Literal::StringLiteral(
                        value.trim().to_owned(),
                    ))),
                }])
            }
            _ => params.clone(),
        };
        let mut values = hir::normalize_annotation_params(&normalized_params);
        if values.is_empty() && valid_string {
            return Ok(None);
        }
        let allowed = values.keys().all(|key| {
            key == "value"
                || (source != HttpParamKind::Body && key == "name")
                || (source == HttpParamKind::Path && key == "path")
        });
        if !valid_string || values.len() != 1 || !allowed {
            return Err(format!(
                "HTTP binding '{owner}' requires one string name, positional or named"
            ));
        }
        Ok(values
            .remove("name")
            .or_else(|| values.remove("value"))
            .or_else(|| values.remove("path")))
    }

    pub(crate) fn wire_name(
        &self,
        raw: &str,
        annotations: &[hir::Annotation],
        container: &[hir::Annotation],
    ) -> String {
        hir::field_rename(annotations)
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| hir::effective_wire_name(raw, annotations, container))
    }
}
