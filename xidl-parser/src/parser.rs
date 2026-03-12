use crate::error::ParserResult;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub struct ParseContext<'a> {
    pub source: &'a [u8],
    pub symbols: HashMap<String, String>,
    doc_consumed: HashSet<usize>,
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            symbols: HashMap::new(),
            doc_consumed: HashSet::new(),
        }
    }

    pub fn node_text(&self, node: &Node) -> ParserResult<&str> {
        Ok(node.utf8_text(self.source)?)
    }

    pub fn take_doc_comment(&mut self, node: &Node) -> Option<String> {
        let start = node.start_byte();
        if self.doc_consumed.contains(&start) {
            return None;
        }
        let doc = self.extract_doc_comment(start);
        if doc.is_some() {
            self.doc_consumed.insert(start);
        }
        doc
    }

    fn extract_doc_comment(&self, start: usize) -> Option<String> {
        if start == 0 {
            return None;
        }
        let src = self.source;
        let mut line_end = if start > 0 && src[start - 1] == b'\n' {
            start - 1
        } else {
            start
        };
        let mut lines = Vec::new();
        let mut first = true;
        loop {
            let mut line_start = 0;
            if line_end > 0 {
                let mut i = line_end;
                while i > 0 && src[i - 1] != b'\n' {
                    i -= 1;
                }
                line_start = i;
            }
            let mut line = &src[line_start..line_end];
            if line.ends_with(b"\r") {
                line = &line[..line.len() - 1];
            }
            if line.iter().all(|b| b.is_ascii_whitespace()) {
                if first {
                    if line_start == 0 {
                        break;
                    }
                    line_end = line_start - 1;
                    first = false;
                    continue;
                }
                break;
            }
            first = false;
            let mut idx = 0;
            while idx < line.len() && line[idx].is_ascii_whitespace() {
                idx += 1;
            }
            let trimmed = &line[idx..];
            if trimmed.starts_with(b"///") {
                let mut content = &trimmed[3..];
                if content.first() == Some(&b' ') {
                    content = &content[1..];
                }
                lines.push(String::from_utf8_lossy(content).to_string());
                if line_start == 0 {
                    break;
                }
                line_end = line_start - 1;
                continue;
            }
            break;
        }
        if lines.is_empty() {
            None
        } else {
            lines.reverse();
            Some(lines.join("\n"))
        }
    }
}

pub trait FromTreeSitter<'a>: Sized {
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self>;
}

impl<'a> FromTreeSitter<'a> for String {
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self> {
        Ok(context.node_text(&node)?.to_string())
    }
}

impl<'a, T> FromTreeSitter<'a> for Box<T>
where
    T: FromTreeSitter<'a>,
{
    fn from_node(node: Node<'a>, context: &mut ParseContext<'a>) -> ParserResult<Self> {
        Ok(Box::new(T::from_node(node, context)?))
    }
}

pub fn parser_text(text: &str) -> ParserResult<crate::typed_ast::Specification> {
    use crate::typed_ast::Specification;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_idl::language()).unwrap();

    let tree = parser.parse(text, None).ok_or_else(|| {
        crate::error::ParseError::TreeSitterError("Failed to parse text".to_string())
    })?;

    let root_node = tree.root_node();
    if root_node.has_error() {
        return Err(crate::error::ParseError::TreeSitterError(
            "Failed to parse text".to_string(),
        ));
    }
    let mut context = ParseContext::new(text.as_bytes());

    Specification::from_node(root_node, &mut context)
}

#[cfg(test)]
mod tests {
    use super::parser_text;
    use crate::typed_ast::{
        AnnotationAppl, AnnotationName, AnnotationParams, Definition, TemplateTypeSpec,
        TypeDclInner, TypeDeclaratorInner, TypeSpec,
    };

    #[test]
    fn parse_template_type_spec() {
        let typed = parser_text(
            r#"
            module m {
                typedef Vec<long> MyVec;
            };
            "#,
        )
        .expect("parse should succeed");

        let module = match &typed.0[0] {
            Definition::ModuleDcl(module) => module,
            other => panic!("expected module, got {other:?}"),
        };
        let type_dcl = match &module.definition[0] {
            Definition::TypeDcl(type_dcl) => type_dcl,
            other => panic!("expected type declaration, got {other:?}"),
        };
        let typedef = match &type_dcl.decl {
            TypeDclInner::TypedefDcl(typedef) => typedef,
            other => panic!("expected typedef, got {other:?}"),
        };
        let template = match &typedef.decl.ty {
            TypeDeclaratorInner::TemplateTypeSpec(TemplateTypeSpec::TemplateType(template)) => {
                template
            }
            other => panic!("expected template_type, got {other:?}"),
        };
        assert_eq!(template.ident.0, "Vec");
        assert_eq!(template.args.len(), 1);
        assert!(matches!(
            template.args[0],
            TypeSpec::SimpleTypeSpec(crate::typed_ast::SimpleTypeSpec::BaseTypeSpec(
                crate::typed_ast::BaseTypeSpec::IntegerType(_)
            ))
        ));
    }

    #[test]
    fn parse_doc_comments_as_doc_annotation() {
        let typed = parser_text(
            r#"
            /// module doc
            module m {
                /// struct doc
                struct S {
                    /// field doc
                    long x;
                };
            };
            "#,
        )
        .expect("parse should succeed");

        let module = match &typed.0[0] {
            Definition::ModuleDcl(module) => module,
            other => panic!("expected module, got {other:?}"),
        };
        assert_has_doc(&module.annotations, "\"module doc\"");

        let type_dcl = match &module.definition[0] {
            Definition::TypeDcl(type_dcl) => type_dcl,
            other => panic!("expected type declaration, got {other:?}"),
        };
        assert_has_doc(&type_dcl.annotations, "\"struct doc\"");

        let struct_def = match &type_dcl.decl {
            TypeDclInner::ConstrTypeDcl(crate::typed_ast::ConstrTypeDcl::StructDcl(
                crate::typed_ast::StructDcl::StructDef(def),
            )) => def,
            other => panic!("expected struct def, got {other:?}"),
        };
        let member = &struct_def.member[0];
        assert_has_doc(&member.annotations, "\"field doc\"");
    }

    fn assert_has_doc(annotations: &[AnnotationAppl], expected: &str) {
        let doc = annotations.iter().find_map(|anno| match &anno.name {
            AnnotationName::Builtin(name) if name == "doc" => match &anno.params {
                Some(AnnotationParams::Raw(raw)) => Some(raw.as_str()),
                _ => None,
            },
            _ => None,
        });
        assert_eq!(doc, Some(expected));
    }
}
