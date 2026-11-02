use std::convert::Infallible;

use xidl_parser::hir;

#[cfg(test)]
mod tests;

/// Reads compiler-collected `///` documentation from semantic metadata.
pub fn doc_lines_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    for annotation in annotations {
        let hir::Annotation::Builtin { name, params } = annotation else {
            continue;
        };
        if !name.eq_ignore_ascii_case("doc") {
            continue;
        }
        if let Some(hir::AnnotationParams::Raw(raw)) = params {
            if let Some(text) = parse_raw_doc(raw) {
                return text.lines().map(|line| line.to_string()).collect();
            }
        }
        return Vec::new();
    }
    Vec::new()
}

fn parse_raw_doc(raw: &str) -> Option<String> {
    let text = raw.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            escaped = false;
        } else {
            match character {
                '\\' => escaped = true,
                '"' => return None,
                _ => {}
            }
        }
    }
    (!escaped).then(|| unescape_doc(text))
}

fn unescape_doc(value: &str) -> String {
    match unescape_zero_copy::unescape(doc_escape_sequence, value) {
        Ok(text) => text.into_owned(),
        Err(never) => match never {},
    }
}

fn doc_escape_sequence(value: &str) -> Result<(char, &str), Infallible> {
    // Decode the compiler's quoting once, preserving literal Markdown escapes.
    Ok(unescape_zero_copy::default_escape_sequence(value).unwrap_or(('\\', value)))
}
