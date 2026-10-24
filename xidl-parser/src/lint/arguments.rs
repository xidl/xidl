use std::collections::BTreeMap;

use crate::{hir, typed_ast as ast};

use super::{AnnotationDefinition, AnnotationKind, ArgumentShape, Site};

struct Arguments {
    values: BTreeMap<String, String>,
    positional: usize,
    named: usize,
    strings_only: bool,
}

impl Arguments {
    fn read(annotation: &ast::AnnotationAppl) -> Result<Self, String> {
        let mut result = Self {
            values: BTreeMap::new(),
            positional: 0,
            named: 0,
            strings_only: true,
        };
        match &annotation.params {
            None => {}
            Some(ast::AnnotationParams::Params(params)) => {
                for param in params {
                    let (key, expression) = match param {
                        ast::AnnotationApplParam::Positional(expr) => {
                            result.positional += 1;
                            ("value".to_string(), expr)
                        }
                        ast::AnnotationApplParam::Named { ident, value } => {
                            result.named += 1;
                            (ident.0.to_ascii_lowercase(), value)
                        }
                    };
                    let normalized = hir::normalize_annotation_params(
                        &hir::AnnotationParams::ConstExpr(expression.clone().into()),
                    );
                    result.strings_only &= matches!(
                        hir::ConstExpr::from(expression.clone()),
                        hir::ConstExpr::Literal(hir::Literal::StringLiteral(_))
                    );
                    let value = normalized.get("value").cloned().unwrap_or_default();
                    if result.values.insert(key.clone(), value).is_some()
                        && (key != "value" || result.named > 0)
                    {
                        return Err(format!("duplicate annotation argument `{key}`"));
                    }
                }
                if result.named > 0 && result.positional > 0 {
                    return Err("cannot mix named and positional annotation arguments".to_string());
                }
            }
            Some(ast::AnnotationParams::Raw(raw)) => {
                let raw = crate::parser::raw::RawArguments::without_comments(raw);
                for part in crate::parser::raw::RawArguments::split(&raw, ',') {
                    let pair = crate::parser::raw::RawArguments::split(part, '=');
                    let (key, value) = match pair.as_slice() {
                        [value] => {
                            result.positional += 1;
                            ("value", *value)
                        }
                        [key, value] => {
                            result.named += 1;
                            (*key, *value)
                        }
                        _ => return Err("invalid annotation argument".to_string()),
                    };
                    result.strings_only &=
                        value.starts_with('"') && value.ends_with('"') && value.len() >= 2;
                    let normalized = hir::normalize_annotation_params(&hir::AnnotationParams::Raw(
                        value.to_string(),
                    ));
                    let value = normalized
                        .get("value")
                        .cloned()
                        .unwrap_or_else(|| value.to_string());
                    let key = key.to_ascii_lowercase();
                    if result.values.insert(key.clone(), value).is_some()
                        && (key != "value" || result.named > 0)
                    {
                        return Err(format!("duplicate annotation argument `{key}`"));
                    }
                }
            }
        }
        if result.named > 0 && result.positional > 0 {
            return Err("cannot mix named and positional annotation arguments".to_string());
        }
        Ok(result)
    }

    fn keys(&self, required: &[&str], optional: &[&str]) -> Result<(), String> {
        for key in required {
            if !self.values.contains_key(*key) {
                return Err(format!("requires `{key}`"));
            }
        }
        for key in self.values.keys() {
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                return Err(format!("unknown argument `{key}`"));
            }
        }
        Ok(())
    }

    fn single(&self, aliases: &[&str], optional: bool) -> Result<(), String> {
        if optional && self.values.is_empty() {
            return Ok(());
        }
        if self.values.len() != 1 || self.positional > 1 {
            return Err("requires exactly one value".to_string());
        }
        if self
            .values
            .keys()
            .all(|key| key == "value" || aliases.contains(&key.as_str()))
        {
            Ok(())
        } else {
            Err(format!(
                "expected a value or one of: {}",
                aliases.join(", ")
            ))
        }
    }

    fn location(&self) -> Result<(), String> {
        self.keys(&["in", "name"], &[])?;
        let location = self.values.get("in").map(String::as_str).unwrap_or("");
        if !["header", "query", "cookie"]
            .iter()
            .any(|name| name.eq_ignore_ascii_case(location))
        {
            return Err("`in` must be header, query or cookie".to_string());
        }
        if self.values.get("name").is_none_or(|name| name.is_empty()) {
            return Err("requires a non-empty `name`".to_string());
        }
        Ok(())
    }

    fn strings(&self) -> Result<(), String> {
        if self.strings_only {
            Ok(())
        } else {
            Err("requires string literal arguments".to_string())
        }
    }
}

impl AnnotationDefinition {
    pub(super) fn check_arguments(
        &self,
        annotation: &ast::AnnotationAppl,
        site: Site,
    ) -> Result<(), String> {
        let args = Arguments::read(annotation)?;
        match self.arguments {
            ArgumentShape::Marker => args.keys(&[], &[]),
            ArgumentShape::Value => args.single(&[], false),
            ArgumentShape::OptionalValue => args.single(&[], true),
            ArgumentShape::String(aliases) => {
                args.single(aliases, false)?;
                args.strings()
            }
            ArgumentShape::OptionalString(aliases) => {
                args.single(aliases, true)?;
                args.strings()
            }
            ArgumentShape::Named { required, optional } => args.keys(required, optional),
            ArgumentShape::Version if annotation.params.is_none() => Ok(()),
            ArgumentShape::Version | ArgumentShape::ApiKey => {
                args.location()?;
                args.strings()
            }
            ArgumentShape::Http if site == Site::Exception => {
                args.single(&["status"], false)?;
                crate::rest_hir::HttpException::status(&[annotation.clone().into()], "annotation")
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            ArgumentShape::Http => args.keys(&[], &[]),
            ArgumentShape::Values if annotation.params.is_some() => Ok(()),
            ArgumentShape::Values => Err("requires at least one value".to_string()),
            ArgumentShape::Cors => {
                if args.named > 0 {
                    return Err("requires positional origin strings".to_string());
                }
                args.strings()?;
                crate::rest_hir::semantics::effective_cors(&[], &[annotation.clone().into()])
                    .map(|_| ())
            }
            ArgumentShape::Deprecated => {
                if args.positional > 0 {
                    args.single(&[], false)?;
                } else {
                    args.keys(&[], &["since", "after"])?;
                }
                crate::rest_hir::semantics::deprecated_info(&[annotation.clone().into()])
                    .map(|_| ())
            }
            ArgumentShape::Replaces => match &annotation.params {
                Some(ast::AnnotationParams::Params(params))
                    if matches!(params.as_slice(),
                    [ast::AnnotationApplParam::Positional(expr)] if matches!(
                        hir::ConstExpr::from(expr.clone()),
                        hir::ConstExpr::ScopedName(_)
                            | hir::ConstExpr::Literal(hir::Literal::StringLiteral(_))
                    )) =>
                {
                    Ok(())
                }
                _ => Err(
                    "requires the earlier member name, for example `@replaces(text)`".to_string(),
                ),
            },
            ArgumentShape::Lifecycle => match &annotation.params {
                Some(ast::AnnotationParams::Params(params))
                    if matches!(params.as_slice(),
                    [ast::AnnotationApplParam::Positional(expr)] if matches!(hir::ConstExpr::from(expr.clone()), hir::ConstExpr::ScopedName(_))) =>
                {
                    Ok(())
                }
                _ => Err(
                    "requires one scoped version reference, for example `Versions::v2`".to_string(),
                ),
            },
            ArgumentShape::Grammar if annotation.builtin.is_some() => Ok(()),
            ArgumentShape::Grammar => {
                Err("arguments do not match the built-in IDL grammar".to_string())
            }
        }?;
        if self.kind == AnnotationKind::Upgrade {
            let annotations = [annotation.clone().into()];
            let protocol = crate::rest_hir::semantics::parse_upgrade_protocol(&annotations)?
                .ok_or_else(|| "requires a protocol".to_string())?;
            let mode = crate::rest_hir::semantics::classify_upgrade_protocol(&protocol)?;
            crate::rest_hir::semantics::parse_websocket_config(&annotations, mode)?;
        }
        Ok(())
    }
}
