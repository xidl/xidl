//! Raw annotation-argument text handling: `key = value` splitting,
//! quote/escape normalization and const-expression rendering.

pub(super) fn parse_raw_annotation_params(raw: &str) -> Vec<(String, String)> {
    let raw = crate::parser::raw::RawArguments::without_comments(raw);
    crate::parser::raw::RawArguments::split(&raw, ',')
        .into_iter()
        .map(|part| {
            if let Some((key, value)) = part.split_once('=') {
                let value = trim_annotation_quotes(value.trim())
                    .unwrap_or_else(|| value.trim().to_string());
                (key.trim().to_string(), unescape_param_value(&value))
            } else {
                let value =
                    trim_annotation_quotes(part.trim()).unwrap_or_else(|| part.trim().to_string());
                ("value".to_string(), unescape_param_value(&value))
            }
        })
        .collect()
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

pub(super) fn trim_annotation_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    let first = value.chars().next().unwrap();
    let last = value.chars().last().unwrap();
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

impl super::VersionParameter {
    pub(crate) fn parse(params: &super::AnnotationParams) -> Result<Self, String> {
        use super::{AnnotationParams, ConstExpr, HttpParameterLocation, Literal};
        let valid_shape = match params {
            AnnotationParams::Params(values) => {
                values.len() == 2
                    && values.iter().all(|param| {
                        matches!(
                            param.value,
                            Some(ConstExpr::Literal(Literal::StringLiteral(_)))
                        )
                    })
            }
            AnnotationParams::Raw(raw) => {
                let raw = crate::parser::raw::RawArguments::without_comments(raw);
                let parts = crate::parser::raw::RawArguments::split(&raw, ',');
                parts.len() == 2 && parts.iter().all(|part| {
                    let pair = crate::parser::raw::RawArguments::split(part, '=');
                    matches!(pair.as_slice(), [_, value] if value.starts_with('"') && value.ends_with('"'))
                })
            }
            _ => false,
        };
        if !valid_shape {
            return Err("requires exactly in=... and name=... string arguments".to_string());
        }
        let values = super::normalize_annotation_params(params);
        if values.len() != 2 || values.keys().any(|key| key != "in" && key != "name") {
            return Err(
                "requires exactly in=... and name=... without duplicate arguments".to_string(),
            );
        }
        let location = values
            .get("in")
            .and_then(|value| HttpParameterLocation::parse(value))
            .ok_or_else(|| "in must be one of header|query|cookie".to_string())?;
        let name = values
            .get("name")
            .cloned()
            .ok_or_else(|| "requires non-empty name=...".to_string())?;
        let parameter = Self { location, name };
        parameter.validate()?;
        Ok(parameter)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.location.valid_name(&self.name) {
            return Err(format!(
                "invalid {} parameter name {:?}",
                self.location.as_str(),
                self.name
            ));
        }
        if self.location == super::HttpParameterLocation::Header {
            let name = self.name.to_ascii_lowercase();
            if matches!(
                name.as_str(),
                "accept"
                    | "content-type"
                    | "content-length"
                    | "content-encoding"
                    | "content-range"
                    | "transfer-encoding"
                    | "host"
                    | "connection"
                    | "upgrade"
                    | "trailer"
                    | "te"
                    | "cookie"
                    | "set-cookie"
            ) || name.starts_with("sec-websocket-")
            {
                return Err(format!(
                    "version header {:?} is reserved for HTTP transport metadata",
                    self.name
                ));
            }
        }
        Ok(())
    }
}
