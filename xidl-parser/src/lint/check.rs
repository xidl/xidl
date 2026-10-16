use std::ops::Range;

use crate::typed_ast::{AnnotationAppl, AnnotationName};
use serde::Serialize;

use super::{AnnotationDefinition, BUILTIN_ANNOTATIONS, Site};

/// Stable identifier of an annotation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCode {
    /// An unknown name is within edit distance two of a built-in name.
    MisspelledAnnotation,
    /// A built-in annotation targets an unsupported declaration.
    MisplacedAnnotation,
    /// Arguments do not satisfy the annotation's contract.
    BadAnnotationArgs,
    /// An unknown annotation may be interpreted by an external plugin.
    UnknownAnnotation,
}

impl DiagnosticCode {
    /// Stable spelling used by command-line output and editor integrations.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MisspelledAnnotation => "misspelled-annotation",
            Self::MisplacedAnnotation => "misplaced-annotation",
            Self::BadAnnotationArgs => "bad-annotation-args",
            Self::UnknownAnnotation => "unknown-annotation",
        }
    }
}

/// Diagnostic severity before a caller applies its warning policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The annotation contract is invalid.
    Error,
    /// The annotation is not known to the built-in vocabulary.
    Warning,
}

/// A source-located annotation diagnostic, with no IO or rendering dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable rule identifier.
    pub code: DiagnosticCode,
    /// Severity independent of a CLI's `--deny warnings` policy.
    pub severity: Severity,
    /// Half-open byte range in the parsed source.
    pub span: Range<usize>,
    /// Description of the failing annotation contract.
    pub message: String,
    /// Suggested spelling, when a nearby built-in name exists.
    pub suggestion: Option<String>,
}

#[derive(Default)]
pub(super) struct Checker {
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
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
        let Some(span) = annotation.span.clone() else {
            return;
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
            self.unknown(name, span);
            return;
        };
        if !definition.allows(site) {
            self.diagnostics.push(Diagnostic {
                code: DiagnosticCode::MisplacedAnnotation,
                severity: Severity::Error,
                span: span.clone(),
                message: format!("@{name} is not allowed on {site:?}"),
                suggestion: None,
            });
        }
        if let Err(reason) = definition.check_arguments(annotation, site) {
            self.diagnostics.push(Diagnostic {
                code: DiagnosticCode::BadAnnotationArgs,
                severity: Severity::Error,
                span,
                message: format!("@{name}: {reason}"),
                suggestion: None,
            });
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
        });
    }
}
