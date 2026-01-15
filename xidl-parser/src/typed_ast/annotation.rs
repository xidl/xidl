use super::*;
use xidl_derive::Parser;

#[derive(Debug, Clone)]
pub struct AnnotationAppl {
    pub name: AnnotationName,
    pub params: Option<AnnotationParams>,
}

#[derive(Debug, Clone)]
pub enum AnnotationName {
    ScopedName(ScopedName),
    Builtin(String),
}

#[derive(Debug, Clone)]
pub enum AnnotationParams {
    ConstExpr(ConstExpr),
    Params(Vec<AnnotationApplParam>),
    Raw(String),
}

#[derive(Debug, Clone, Parser)]
pub struct AnnotationApplParam {
    pub ident: Identifier,
    pub value: Option<ConstExpr>,
}

impl<'a> crate::parser::FromTreeSitter<'a> for AnnotationAppl {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_derive::node_id!("annotation_appl"));
        let raw = ctx.node_text(&node)?.trim();
        if raw.starts_with("//@") {
            return Err(crate::error::ParseError::UnexpectedNode(
                "extend annotation is not supported".to_string(),
            ));
        }

        let mut custom_body = None;
        let mut builtin_body = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_derive::node_id!("annotation_appl_custom_body") => {
                    custom_body = Some(ch);
                }
                xidl_derive::node_id!("annotation_appl_builtin_body") => {
                    builtin_body = Some(ch);
                }
                xidl_derive::node_id!("extend_annotation_appl") => {
                    return Err(crate::error::ParseError::UnexpectedNode(
                        "extend annotation is not supported".to_string(),
                    ));
                }
                _ => {}
            }
        }

        if let Some(custom_body) = custom_body {
            let mut scoped_name = None;
            let mut params = None;
            for ch in custom_body.children(&mut custom_body.walk()) {
                match ch.kind_id() {
                    xidl_derive::node_id!("scoped_name") => {
                        scoped_name = Some(ScopedName::from_node(ch, ctx)?);
                    }
                    xidl_derive::node_id!("annotation_appl_params") => {
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
            });
        }

        let source = builtin_body
            .map(|node| ctx.node_text(&node))
            .transpose()?
            .unwrap_or(raw);
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
        })
    }
}

impl<'a> crate::parser::FromTreeSitter<'a> for AnnotationParams {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(
            node.kind_id(),
            xidl_derive::node_id!("annotation_appl_params")
        );

        let mut const_expr = None;
        let mut params = vec![];
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_derive::node_id!("const_expr") => {
                    const_expr = Some(ConstExpr::from_node(ch, ctx)?);
                }
                xidl_derive::node_id!("annotation_appl_param") => {
                    params.push(AnnotationApplParam::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }

        if let Some(const_expr) = const_expr {
            return Ok(Self::ConstExpr(const_expr));
        }
        if !params.is_empty() {
            return Ok(Self::Params(params));
        }

        Err(crate::error::ParseError::UnexpectedNode(
            "annotation_appl_params missing content".to_string(),
        ))
    }
}
