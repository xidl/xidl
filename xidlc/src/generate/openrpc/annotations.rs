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

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

pub(super) fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        matches!(annotation, hir::Annotation::Optional { .. })
            || annotation_name(annotation)
                .map(|name| name.eq_ignore_ascii_case("optional"))
                .unwrap_or(false)
    })
}

pub(super) fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

pub(super) fn stream_kind_from_annotations(annotations: &[hir::Annotation]) -> Option<StreamKind> {
    let mut out = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = match name.to_ascii_lowercase().as_str() {
            "server_stream" => Some(StreamKind::Server),
            "client_stream" => Some(StreamKind::Client),
            "bidi_stream" => Some(StreamKind::Bidi),
            _ => None,
        };
        let Some(current) = current else {
            continue;
        };
        match out {
            None => out = Some(current),
            Some(prev) if prev == current => {}
            Some(_) => panic!("@server_stream/@client_stream/@bidi_stream are mutually exclusive"),
        }
    }
    out
}

pub(super) fn stream_extension(
    kind: StreamKind,
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> Value {
    let _ = (module_path, interface_name, method_name);
    stream_extension_direct(kind)
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
