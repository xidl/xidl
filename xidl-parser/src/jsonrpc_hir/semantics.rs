use std::collections::HashSet;

use crate::error::{ParseError, ParserResult};
use crate::hir;

use super::JsonRpcMethodKind;

#[cfg(test)]
mod tests;

pub(super) fn param_is_input(attr: Option<&hir::ParamAttribute>) -> bool {
    !matches!(attr.map(|value| value.0.as_str()), Some("out"))
}

pub(super) fn param_is_output(attr: Option<&hir::ParamAttribute>) -> bool {
    matches!(attr.map(|value| value.0.as_str()), Some("out" | "inout"))
}

pub(super) fn stream_kind(
    annotations: &[hir::Annotation],
) -> ParserResult<Option<JsonRpcMethodKind>> {
    let mut out = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("server_stream") {
            Some(JsonRpcMethodKind::ServerStream)
        } else if name.eq_ignore_ascii_case("client_stream") {
            Some(JsonRpcMethodKind::ClientStream)
        } else if name.eq_ignore_ascii_case("bidi_stream") {
            Some(JsonRpcMethodKind::BidiStream)
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
                return Err(ParseError::Message(
                    "@server_stream/@client_stream/@bidi_stream are mutually exclusive".to_string(),
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
) -> ParserResult<()> {
    let getter_conflict = user_ops.contains(getter);
    let setter_conflict = !setter.is_empty() && user_ops.contains(setter);
    if getter_conflict || setter_conflict {
        let conflict = if setter.is_empty() {
            format!("`{getter}`")
        } else {
            format!("`{getter}` or `{setter}`")
        };
        return Err(ParseError::Message(format!(
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
