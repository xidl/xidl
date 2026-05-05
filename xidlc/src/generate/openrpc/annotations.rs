use serde_json::{Value, json};
use xidl_parser::hir;

use crate::generate::utils::doc_lines_from_annotations;

#[derive(Copy, Clone, Eq, PartialEq)]
pub(super) enum StreamKind {
    Server,
    Client,
    Bidi,
}

pub(super) fn doc_text(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

pub(super) fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| match annotation {
        hir::Annotation::Builtin { name, .. } => name.eq_ignore_ascii_case(target),
        hir::Annotation::ScopedName { name, .. } => name
            .name
            .last()
            .map(|value| value.eq_ignore_ascii_case(target))
            .unwrap_or(false),
        _ => false,
    })
}

pub(super) fn stream_extension_direct(kind: StreamKind) -> Value {
    json!({
        "mode": stream_mode_name(kind),
        "codec": "json",
        "delivery": "direct",
    })
}

fn stream_mode_name(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Server => "server",
        StreamKind::Client => "client",
        StreamKind::Bidi => "bidi",
    }
}
