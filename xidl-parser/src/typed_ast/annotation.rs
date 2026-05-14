use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationAppl {
    pub name: AnnotationName,
    pub params: Option<AnnotationParams>,
    pub builtin: Option<BuiltinAnnotation>,
    pub is_extend: bool,
    pub extra: Vec<AnnotationAppl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationName {
    ScopedName(ScopedName),
    Builtin(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationParams {
    Params(Vec<AnnotationApplParam>),
    Raw(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationApplParam {
    Positional(ConstExpr),
    Named { ident: Identifier, value: ConstExpr },
}

impl<'a> crate::parser::FromTreeSitter<'a> for AnnotationAppl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        let kind_id = node.kind_id();
        let is_extend = kind_id == xidl_parser_derive::node_id!("extend_annotation_appl");
        if !is_extend {
            assert_eq!(kind_id, xidl_parser_derive::node_id!("annotation_appl"));
        }
        let raw = ctx.node_text(&node)?.to_string();

        let mut custom_body = None;
        let mut builtin_body = None;
        let mut extra = Vec::new();
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl_custom_body") => {
                    custom_body = Some(ch);
                }
                xidl_parser_derive::node_id!("annotation_appl_builtin_body") => {
                    builtin_body = Some(ch);
                }
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    extra.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }

        if requires_raw_annotation_parse(&raw) {
            return parse_annotation_from_raw(&raw, is_extend, extra);
        }

        if let Some(custom_body) = custom_body {
            let mut scoped_name = None;
            let mut params = None;
            for ch in custom_body.children(&mut custom_body.walk()) {
                match ch.kind_id() {
                    xidl_parser_derive::node_id!("scoped_name") => {
                        scoped_name = Some(ScopedName::from_node(ch, ctx)?);
                    }
                    xidl_parser_derive::node_id!("annotation_appl_params") => {
                        params = Some(AnnotationParams::from_node(ch, ctx)?);
                    }
                    _ => {}
                }
            }
            let scoped_name = scoped_name.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(
                    "annotation_appl_custom_body missing scoped_name".to_string(),
                )
            })?;
            return Ok(Self {
                name: AnnotationName::ScopedName(scoped_name),
                params,
                builtin: None,
                is_extend,
                extra,
            });
        }

        let builtin = builtin_body
            .map(|node| parse_builtin_annotation(node, ctx))
            .transpose()?;
        let source = builtin_body
            .map(|node| ctx.node_text(&node))
            .transpose()?
            .unwrap_or(raw.as_str());
        let source = source.trim().strip_prefix("//@").unwrap_or(source);
        let raw = source.trim().strip_prefix('@').unwrap_or(source).trim();
        let (name, args) = match raw.split_once('(') {
            Some((name, rest)) => {
                let args = rest.strip_suffix(')').unwrap_or(rest).trim();
                (name.trim(), Some(args))
            }
            None => (raw, None),
        };

        Ok(Self {
            name: AnnotationName::Builtin(name.to_string()),
            params: args.map(|value| AnnotationParams::Raw(value.to_string())),
            builtin,
            is_extend,
            extra,
        })
    }
}

fn requires_raw_annotation_parse(raw: &str) -> bool {
    let source = raw.trim();
    let source = source.strip_prefix("//@").unwrap_or(source);
    let source = source.strip_prefix('@').unwrap_or(source);
    let name = source
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(source);
    name.contains('-') || source.contains('[') || source.contains(']')
}

fn parse_annotation_from_raw(
    raw: &str,
    is_extend: bool,
    extra: Vec<AnnotationAppl>,
) -> crate::error::ParserResult<AnnotationAppl> {
    let source = raw.trim();
    let source = source.strip_prefix("//@").unwrap_or(source);
    let source = source.strip_prefix('@').unwrap_or(source).trim();
    let (name, args) = match source.split_once('(') {
        Some((name, rest)) => {
            let args = rest.strip_suffix(')').unwrap_or(rest).trim();
            (name.trim(), Some(args))
        }
        None => (source, None),
    };
    Ok(AnnotationAppl {
        name: AnnotationName::Builtin(name.to_string()),
        params: args.map(|value| AnnotationParams::Raw(value.to_string())),
        builtin: None,
        is_extend,
        extra,
    })
}

impl AnnotationAppl {
    pub fn doc(text: String) -> Self {
        let escaped = escape_doc_text(&text);
        Self {
            name: AnnotationName::Builtin("doc".to_string()),
            params: Some(AnnotationParams::Raw(format!("\"{}\"", escaped))),
            builtin: None,
            is_extend: false,
            extra: Vec::new(),
        }
    }
}

fn escape_doc_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

impl<'a> crate::parser::FromTreeSitter<'a> for AnnotationParams {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("annotation_appl_params")
        );

        let mut params = vec![];
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("const_expr") => {
                    return Ok(Self::Params(vec![AnnotationApplParam::Positional(
                        ConstExpr::from_node(ch, ctx)?,
                    )]));
                }
                xidl_parser_derive::node_id!("annotation_appl_param") => {
                    params.push(AnnotationApplParam::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }

        if !params.is_empty() {
            return Ok(Self::Params(params));
        }

        Err(crate::error::ParseError::UnexpectedNode(
            "annotation_appl_params missing content".to_string(),
        ))
    }
}

impl<'a> crate::parser::FromTreeSitter<'a> for AnnotationApplParam {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_parser_derive::node_id!("annotation_appl_param")
        );

        let mut ident = None;
        let mut value = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("identifier") => {
                    ident = Some(Identifier::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("const_expr") => {
                    value = Some(ConstExpr::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }

        match (ident, value) {
            (None, Some(value)) => Ok(Self::Positional(value)),
            (Some(ident), Some(value)) => Ok(Self::Named { ident, value }),
            _ => Err(crate::error::ParseError::UnexpectedNode(
                "annotation_appl_param missing const_expr".to_string(),
            )),
        }
    }
}
