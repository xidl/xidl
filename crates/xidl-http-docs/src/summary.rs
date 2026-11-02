use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) struct ChangeSummary;

impl ChangeSummary {
    pub fn pair(before: Option<&Value>, after: Option<&Value>) -> (String, String) {
        let previous = Self::describe(before);
        let next = Self::describe(after);
        if before.is_some()
            && after.is_some()
            && (previous == next || before.is_some_and(|value| value.get("routes").is_some()))
        {
            let mut summary = DifferenceSummary::default();
            summary.visit("", (before, after));
            return summary.finish();
        }
        (previous, next)
    }

    pub fn describe(value: Option<&Value>) -> String {
        let Some(value) = value else {
            return "Absent".into();
        };
        if let Some(routes) = value.get("routes").and_then(Value::as_array) {
            return routes
                .iter()
                .filter_map(|route| {
                    Some(format!(
                        "{} {}",
                        route.get(0)?.as_str()?,
                        route.get(1)?.as_str()?
                    ))
                })
                .collect::<Vec<_>>()
                .join(" · ");
        }
        let schema = value.get("schema").unwrap_or(value);
        let mut parts = vec![Self::type_name(schema)];
        if let Some(required) = value.get("required").and_then(Value::as_bool) {
            parts.push(if required { "required" } else { "optional" }.into());
        }
        for key in [
            "enum",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "minItems",
            "maxItems",
            "default",
        ] {
            if let Some(value) = schema.get(key) {
                parts.push(format!("{key}: {value}"));
            }
        }
        for key in ["x-xidl-declared-default", "x-xidl-declared-constraints"] {
            if let Some(value) = schema.get(key) {
                parts.push(format!(
                    "declared {}: {value}",
                    key.trim_start_matches("x-xidl-declared-")
                ));
            }
        }
        parts.join(" · ")
    }

    fn type_name(schema: &Value) -> String {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return reference
                .rsplit('/')
                .next()
                .unwrap_or(reference)
                .replace('.', "::");
        }
        if schema["type"] == "array" {
            return format!("array<{}>", Self::type_name(&schema["items"]));
        }
        schema
            .get("type")
            .map(|value| {
                value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned)
            })
            .unwrap_or_else(|| "Schema".into())
    }
}

#[derive(Default)]
struct DifferenceSummary {
    before: Vec<String>,
    after: Vec<String>,
    count: usize,
}

impl DifferenceSummary {
    fn visit(&mut self, path: &str, values: (Option<&Value>, Option<&Value>)) {
        let (before, after) = values;
        if before == after {
            return;
        }
        match (before, after) {
            (Some(Value::Object(a)), Some(Value::Object(b))) => {
                let mut keys = a
                    .keys()
                    .chain(b.keys())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                keys.sort_by(|a, b| {
                    Self::priority(a)
                        .cmp(&Self::priority(b))
                        .then_with(|| a.cmp(b))
                });
                for key in keys {
                    let path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.visit(&path, (a.get(key), b.get(key)));
                }
            }
            (Some(Value::Array(a)), Some(Value::Array(b))) => {
                for index in 0..a.len().max(b.len()) {
                    self.visit(&format!("{path}[{index}]"), (a.get(index), b.get(index)));
                }
            }
            _ => {
                self.count += 1;
                if self.before.len() < 3 {
                    let parts = path
                        .trim_start_matches("contract.")
                        .split('.')
                        .collect::<Vec<_>>();
                    let label = if parts.len() > 4 {
                        format!(
                            "{}.{}.…{}.{}",
                            parts[0],
                            parts[1],
                            parts[parts.len() - 2],
                            parts[parts.len() - 1]
                        )
                    } else {
                        parts.join(".")
                    };
                    self.before
                        .push(format!("{label}: {}", Self::value(before)));
                    self.after.push(format!("{label}: {}", Self::value(after)));
                }
            }
        }
    }

    fn priority(key: &str) -> usize {
        match key {
            "routes" => 0,
            "type" | "$ref" => 1,
            "required" => 2,
            "enum" => 3,
            "format" => 4,
            _ => 5,
        }
    }

    fn value(value: Option<&Value>) -> String {
        let value = value.map_or_else(|| "Absent".into(), Value::to_string);
        if value.chars().count() > 100 {
            format!("{}…", value.chars().take(100).collect::<String>())
        } else {
            value
        }
    }

    fn finish(self) -> (String, String) {
        let mut before = self.before.join(" · ");
        let mut after = self.after.join(" · ");
        if self.count > 3 {
            let more = format!(" · +{} more differences", self.count - 3);
            before.push_str(&more);
            after.push_str(&more);
        }
        (before, after)
    }
}
