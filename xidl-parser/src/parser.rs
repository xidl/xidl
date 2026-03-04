use crate::error::ParserResult;
use std::collections::HashMap;
use tree_sitter::Node;

pub struct ParseContext<'a> {
    pub source: &'a [u8],
    pub symbols: HashMap<String, String>,
}

impl<'a> ParseContext<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            symbols: HashMap::new(),
        }
    }

    pub fn node_text(&self, node: &Node) -> ParserResult<&str> {
        Ok(node.utf8_text(self.source)?)
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
        Definition, TemplateTypeSpec, TypeDclInner, TypeDeclaratorInner, TypeSpec,
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
}
