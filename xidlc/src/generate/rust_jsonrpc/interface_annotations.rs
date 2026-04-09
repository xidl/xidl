use crate::error::{IdlcError, IdlcResult};
use std::collections::HashSet;
use xidl_parser::hir;

use super::interface_model::{ParamMode, StreamKind};

pub(super) fn stream_mode_name(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Server => "server",
        StreamKind::Client => "client",
        StreamKind::Bidi => "bidi",
    }
}

pub(super) fn param_mode(attr: Option<&hir::ParamAttribute>) -> ParamMode {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamMode::Out,
        Some("inout") => ParamMode::InOut,
        _ => ParamMode::In,
    }
}

pub(super) fn stream_kind_from_annotations(
    annotations: &[hir::Annotation],
) -> IdlcResult<Option<StreamKind>> {
    let mut out = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("server_stream") {
            Some(StreamKind::Server)
        } else if name.eq_ignore_ascii_case("client_stream") {
            Some(StreamKind::Client)
        } else if name.eq_ignore_ascii_case("bidi_stream") {
            Some(StreamKind::Bidi)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        match out {
            None => out = Some(current),
            Some(prev) if prev == current => {}
            Some(_) => {
                return Err(IdlcError::rpc(
                    "@server_stream/@client_stream/@bidi_stream are mutually exclusive",
                ));
            }
        }
    }
    Ok(out)
}

pub(super) fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

pub(super) fn validate_attr_collision(
    user_ops: &HashSet<&str>,
    attr_name: &str,
    getter: &str,
    setter: &str,
) -> IdlcResult<()> {
    let getter_conflict = user_ops.contains(getter);
    let setter_conflict = !setter.is_empty() && user_ops.contains(setter);
    if getter_conflict || setter_conflict {
        let conflict = if setter.is_empty() {
            format!("`{getter}`")
        } else {
            format!("`{getter}` or `{setter}`")
        };
        return Err(IdlcError::fmt(format!(
            "attribute `{attr_name}` conflicts with user-defined operation `{conflict}`"
        )));
    }
    Ok(())
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}
