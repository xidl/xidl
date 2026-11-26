use std::ops::Range;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::typed_ast::{AnnotationAppl, AnnotationName};

use super::{AnnotationDefinition, AnnotationKind, BUILTIN_ANNOTATIONS, Site};

#[derive(Default)]
pub(super) struct Checker {
    diagnostics: Vec<Diagnostic>,
    selected: Option<AnnotationKind>,
}

impl Checker {
    pub(super) fn for_annotation(kind: AnnotationKind) -> Self {
        Self {
            selected: Some(kind),
            ..Self::default()
        }
    }

    pub(super) fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics
            .sort_by_key(|diagnostic| diagnostic.span.start);
        self.diagnostics
    }

    pub(super) fn annotations(&mut self, annotations: &[AnnotationAppl], site: Site) {
        for annotation in annotations {
            self.annotation(annotation, site);
            self.annotations(&annotation.extra, site);
        }
    }

    fn annotation(&mut self, annotation: &AnnotationAppl, site: Site) {
        let span = match &annotation.span {
            Some(span) => span.clone(),
            None if self.selected.is_some() => 0..0,
            None => return,
        };
        let name = match &annotation.name {
            AnnotationName::Builtin(name) => name.clone(),
            AnnotationName::ScopedName(name) => {
                let name: crate::hir::ScopedName = name.clone().into();
                format!(
                    "{}{}",
                    if name.is_root { "::" } else { "" },
                    name.name.join("::")
                )
            }
        };
        if AnnotationDefinition::is_passthrough(&name) {
            return;
        }
        let Some(definition) = AnnotationDefinition::lookup(&name) else {
            if self.selected.is_none() {
                self.unknown(name, span);
            }
            return;
        };
        if self.selected.is_some_and(|kind| kind != definition.kind) {
            return;
        }
        if !definition.allows(site) {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticCode::MisplacedAnnotation,
                span.clone(),
                format!("@{name} is not allowed on {site:?}"),
            ));
        }
        if let Err(reason) = definition.check_arguments(annotation, site) {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticCode::BadAnnotationArgs,
                span,
                format!("@{name}: {reason}"),
            ));
        }
    }

    fn unknown(&mut self, name: String, span: Range<usize>) {
        let lowered = name.to_ascii_lowercase();
        let suggestion = BUILTIN_ANNOTATIONS
            .iter()
            .filter(|entry| entry.name.len().abs_diff(lowered.len()) <= 2)
            .map(|entry| (strsim::levenshtein(&lowered, entry.name), entry.name))
            .filter(|(distance, _)| *distance <= 2)
            .min()
            .map(|(_, name)| name.to_string());
        let (code, severity) = if suggestion.is_some() {
            (DiagnosticCode::MisspelledAnnotation, Severity::Error)
        } else {
            (DiagnosticCode::UnknownAnnotation, Severity::Warning)
        };
        self.diagnostics.push(Diagnostic {
            code,
            severity,
            span,
            message: format!("unknown annotation `@{name}`"),
            suggestion,
            location: None,
        });
    }
}
