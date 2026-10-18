//! Canonical HTTP parameter bindings shared by projection and version facts.

use super::super::semantics::SourceBinding;
use super::super::semantics::{HttpStreamKind, has_annotation, has_optional_annotation};
use super::super::validate::{HttpParamDirection, default_param_source, param_direction};
use super::super::{HttpMethod, HttpParam, HttpParamKind, HttpRoute, RestHirResult};
use crate::hir;
use std::collections::HashSet;

/// Resolves parameter sources with the exact REST projection rules.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterSources {
    default_source: HttpParamKind,
    pub(super) route_path_names: Vec<HashSet<String>>,
    route_query_names: Vec<HashSet<String>>,
}

impl ParameterSources {
    /// Builds a binding context from normalized HTTP routes.
    pub(crate) fn new(
        method: HttpMethod,
        stream_kind: Option<HttpStreamKind>,
        routes: &[HttpRoute],
    ) -> Self {
        Self {
            default_source: if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
                HttpParamKind::Body
            } else {
                default_param_source(method)
            },
            route_path_names: routes
                .iter()
                .map(|route| route.path_params.iter().cloned().collect())
                .collect(),
            route_query_names: routes
                .iter()
                .map(|route| route.query_params.iter().cloned().collect())
                .collect(),
        }
    }

    /// Projects a request parameter, excluding response-only declarations.
    pub(crate) fn request(&self, param: &hir::ParamDcl) -> RestHirResult<Option<HttpParam>> {
        if matches!(
            param_direction(param.attr.as_ref()),
            HttpParamDirection::Out
        ) {
            return Ok(None);
        }
        self.project(param).map(Some)
    }

    /// Projects a response parameter, excluding input-only declarations.
    pub(crate) fn response(&self, param: &hir::ParamDcl) -> RestHirResult<Option<HttpParam>> {
        if matches!(param_direction(param.attr.as_ref()), HttpParamDirection::In) {
            return Ok(None);
        }
        self.project(param).map(Some)
    }

    /// Projects a parameter without changing its direction.
    pub(crate) fn project(&self, param: &hir::ParamDcl) -> RestHirResult<HttpParam> {
        let binding = SourceBinding::read(&param.annotations, &param.declarator.0)?;
        let wire_name = binding.as_ref().map_or_else(
            || hir::field_rename(&param.annotations).unwrap_or_else(|| param.declarator.0.clone()),
            |binding| binding.wire_name(&param.declarator.0, &param.annotations, &[]),
        );
        let kind = match param_direction(param.attr.as_ref()) {
            HttpParamDirection::Out | HttpParamDirection::InOut => binding
                .map(|value| value.source)
                .unwrap_or(HttpParamKind::Body),
            HttpParamDirection::In => binding.map(|value| value.source).unwrap_or_else(|| {
                if self
                    .route_path_names
                    .iter()
                    .all(|set| set.contains(&wire_name))
                {
                    HttpParamKind::Path
                } else if self
                    .route_query_names
                    .iter()
                    .any(|set| set.contains(&wire_name))
                {
                    HttpParamKind::Query
                } else {
                    self.default_source
                }
            }),
        };
        Ok(HttpParam {
            name: param.declarator.0.clone(),
            wire_name,
            ty: param.ty.clone(),
            kind,
            optional: has_optional_annotation(&param.annotations),
            flatten: has_annotation(&param.annotations, "flatten"),
        })
    }
}
