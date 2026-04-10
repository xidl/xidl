use super::{SerializeKind, SerializeVersion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Pragma {
    XidlcSerialize(SerializeKind),
    XidlcVersion(SerializeVersion),
    XidlcPackage(String),
    XidlcOpenApiVersion(String),
    XidlcOpenApiService {
        base_url: String,
        description: Option<String>,
    },
}

pub(crate) fn parse_xidlc_pragma(call: &crate::typed_ast::PreprocCall) -> Option<Pragma> {
    let directive = call.directive.0.as_str();
    if !directive.eq_ignore_ascii_case("#pragma") {
        return None;
    }

    let arg = call.argument.as_ref()?.0.as_str();
    let mut parts = arg.split_whitespace();
    let namespace = parts.next()?;
    if !namespace.eq_ignore_ascii_case("xidlc") {
        return None;
    }

    let token = parts.next()?;
    let rest = parts.collect::<Vec<_>>().join(" ");
    if token.eq_ignore_ascii_case("XCDR1") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr1));
    }
    if token.eq_ignore_ascii_case("XCDR2") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr2));
    }
    if token.eq_ignore_ascii_case("package") {
        return (!rest.is_empty()).then(|| Pragma::XidlcPackage(trim_pragma_value(&rest)));
    }
    if token.eq_ignore_ascii_case("version") {
        return (!rest.is_empty()).then(|| Pragma::XidlcOpenApiVersion(trim_pragma_value(&rest)));
    }
    if token.eq_ignore_ascii_case("service") {
        return parse_pragma_service(&rest).map(|(base_url, description)| {
            Pragma::XidlcOpenApiService {
                base_url,
                description,
            }
        });
    }
    if token.eq_ignore_ascii_case("openapi") {
        return parse_nested_openapi_pragma(&rest);
    }

    token
        .strip_prefix("serialize(")
        .and_then(|value| value.strip_suffix(')'))
        .and_then(parse_serialize_pragma)
}

pub(crate) fn trim_pragma_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn parse_nested_openapi_pragma(rest: &str) -> Option<Pragma> {
    let mut nested = rest.split_whitespace();
    let token = nested.next()?;
    let nested_rest = nested.collect::<Vec<_>>().join(" ");
    if token.eq_ignore_ascii_case("version") && !nested_rest.is_empty() {
        return Some(Pragma::XidlcOpenApiVersion(trim_pragma_value(&nested_rest)));
    }
    if token.eq_ignore_ascii_case("service") {
        return parse_pragma_service(&nested_rest).map(|(base_url, description)| {
            Pragma::XidlcOpenApiService {
                base_url,
                description,
            }
        });
    }
    None
}

fn parse_serialize_pragma(value: &str) -> Option<Pragma> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("XCDR1") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr1));
    }
    if value.eq_ignore_ascii_case("XCDR2") {
        return Some(Pragma::XidlcVersion(SerializeVersion::Xcdr2));
    }
    parse_serialize_kind(value).map(Pragma::XidlcSerialize)
}

fn parse_pragma_service(value: &str) -> Option<(String, Option<String>)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let (base_url, remainder) = if value.starts_with('"') || value.starts_with('\'') {
        let quote = value.chars().next().unwrap();
        let end = value.char_indices().skip(1).find(|(_, ch)| *ch == quote)?.0;
        (value[1..end].to_string(), value[end + 1..].trim())
    } else {
        let mut parts = value.splitn(2, char::is_whitespace);
        (parts.next()?.to_string(), parts.next().unwrap_or("").trim())
    };

    let description = (!remainder.is_empty()).then(|| trim_pragma_value(remainder));
    Some((base_url, description))
}

fn parse_serialize_kind(value: &str) -> Option<SerializeKind> {
    if value.eq_ignore_ascii_case("CDR") {
        Some(SerializeKind::Cdr)
    } else if value.eq_ignore_ascii_case("PLAIN_CDR") {
        Some(SerializeKind::PlainCdr)
    } else if value.eq_ignore_ascii_case("PL_CDR") {
        Some(SerializeKind::PlCdr)
    } else if value.eq_ignore_ascii_case("PLAIN_CDR2") {
        Some(SerializeKind::PlainCdr2)
    } else if value.eq_ignore_ascii_case("DELIMITED_CDR") {
        Some(SerializeKind::DelimitedCdr)
    } else if value.eq_ignore_ascii_case("PL_CDR2") {
        Some(SerializeKind::PlCdr2)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
