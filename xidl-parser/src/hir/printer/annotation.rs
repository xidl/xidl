use super::super::*;
use super::IdlPrinter;

impl IdlPrinter {
    pub(super) fn render_annotations_inline(annotations: &[Annotation]) -> String {
        annotations
            .iter()
            .filter(|annotation| !Self::is_doc_annotation(annotation))
            .map(Self::render_annotation)
            .map(|value| format!("{value} "))
            .collect()
    }

    pub(super) fn render_annotation(annotation: &Annotation) -> String {
        match annotation {
            Annotation::Id { value } => format!("@id({value})"),
            Annotation::Key { value } => Self::render_optional_value("key", value.as_deref()),
            Annotation::AutoId { value } => Self::render_optional_value("autoid", value.as_deref()),
            Annotation::Optional { value } => {
                Self::render_optional_value("optional", value.as_deref())
            }
            Annotation::Position { value } => format!("@position({value})"),
            Annotation::Value { value } => format!("@value({value})"),
            Annotation::Extensibility { kind } => format!("@extensibility({kind})"),
            Annotation::Final => "@final".to_string(),
            Annotation::Appendable => "@appendable".to_string(),
            Annotation::Mutable => "@mutable".to_string(),
            Annotation::MustUnderstand { value } => {
                Self::render_optional_value("must_understand", value.as_deref())
            }
            Annotation::Default { value } => format!("@default({value})"),
            Annotation::Range { min, max } => format!("@range(min = {min}, max = {max})"),
            Annotation::Min { value } => format!("@min({value})"),
            Annotation::Max { value } => format!("@max({value})"),
            Annotation::Unit { value } => format!("@unit({value})"),
            Annotation::BitBound { value } => format!("@bit_bound({value})"),
            Annotation::External { value } => {
                Self::render_optional_value("external", value.as_deref())
            }
            Annotation::Nested { value } => Self::render_optional_value("nested", value.as_deref()),
            Annotation::Verbatim {
                language,
                placement,
                text,
            } => {
                let mut values = Vec::new();
                if let Some(language) = language {
                    values.push(match language.as_str() {
                        "c" => "C".to_string(),
                        "c++" => "C++".to_string(),
                        "java" => "Java".to_string(),
                        "idl" => "IDL".to_string(),
                        _ => language.clone(),
                    });
                }
                if let Some(placement) = placement {
                    values.push(placement.clone());
                }
                values.push(text.clone());
                format!("@verbatim({})", values.join(", "))
            }
            Annotation::Service { platform } => {
                Self::render_optional_value("service", platform.as_deref())
            }
            Annotation::Oneway { value } => Self::render_optional_value("oneway", value.as_deref()),
            Annotation::Ami { value } => Self::render_optional_value("ami", value.as_deref()),
            Annotation::HashId { value } => Self::render_optional_value("hashid", value.as_deref()),
            Annotation::DefaultNested { value } => {
                Self::render_optional_value("default_nested", value.as_deref())
            }
            Annotation::IgnoreLiteralNames { value } => {
                Self::render_optional_value("ignore_literal_names", value.as_deref())
            }
            Annotation::TryConstruct { value } => {
                Self::render_optional_value("try_construct", value.as_deref())
            }
            Annotation::NonSerialized { value } => {
                Self::render_optional_value("non_serialized", value.as_deref())
            }
            Annotation::DataRepresentation { kinds } => {
                format!("@data_representation({})", kinds.join(" | "))
            }
            Annotation::Topic { name, platform } => {
                let mut values = Vec::new();
                if let Some(name) = name {
                    values.push(name.clone());
                }
                if let Some(platform) = platform {
                    values.push(platform.clone());
                }
                if values.is_empty() {
                    "@topic".to_string()
                } else {
                    format!("@topic({})", values.join(", "))
                }
            }
            Annotation::Choice => "@choice".to_string(),
            Annotation::Empty => "@empty".to_string(),
            Annotation::DdsService => "@dds_service".to_string(),
            Annotation::DdsRequestTopic { name } => format!("@dds_request_topic({name})"),
            Annotation::DdsReplyTopic { name } => format!("@dds_reply_topic({name})"),
            Annotation::Builtin { name, params } => {
                Self::render_named_annotation(name, params.as_ref())
            }
            Annotation::ScopedName { name, params } => {
                Self::render_named_annotation(&Self::render_scoped(name), params.as_ref())
            }
            Annotation::DefaultLiteral => "@default_literal".to_string(),
            Annotation::Rename { name } => format!("@rename({})", Self::quote(name)),
            Annotation::RenameAll { rule } => {
                format!("@rename_all({})", Self::quote(rule.as_str()))
            }
            Annotation::Skip => "@skip".to_string(),
            Annotation::Version => "@version".to_string(),
            Annotation::VersionParameter(parameter) => format!(
                "@version(in = {}, name = {})",
                Self::quote(parameter.location.as_str()),
                Self::quote(&parameter.name)
            ),
            Annotation::Added { version } => format!("@added({})", Self::render_scoped(version)),
            Annotation::Removed { version } => {
                format!("@removed({})", Self::render_scoped(version))
            }
        }
    }

    pub(super) fn render_optional_value(name: &str, value: Option<&str>) -> String {
        match value {
            Some(value) => format!("@{name}({value})"),
            None => format!("@{name}"),
        }
    }

    pub(super) fn render_named_annotation(name: &str, params: Option<&AnnotationParams>) -> String {
        let Some(params) = params else {
            return format!("@{name}");
        };
        let value = match params {
            AnnotationParams::ConstExpr(value) => Self::render_expr(value),
            AnnotationParams::Positional(values) => values
                .iter()
                .map(Self::render_expr)
                .collect::<Vec<_>>()
                .join(", "),
            AnnotationParams::Params(values) => values
                .iter()
                .map(|parameter| match &parameter.value {
                    Some(value) => format!("{} = {}", parameter.ident, Self::render_expr(value)),
                    None => parameter.ident.clone(),
                })
                .collect::<Vec<_>>()
                .join(", "),
            AnnotationParams::Raw(value) => crate::parser::raw::RawArguments::canonical(value),
        };
        format!("@{name}({value})")
    }
}
