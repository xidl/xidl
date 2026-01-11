use super::*;

pub enum Declarator {
    SimpleDeclarator(SimpleDeclarator),
    ArrayDeclarator(ArrayDeclarator),
}

pub struct ArrayDeclarator {
    pub ident: String,
    pub len: Vec<ConstExpr>,
}

pub struct SimpleDeclarator(pub String);

impl From<crate::typed_ast::Declarator> for Declarator {
    fn from(value: crate::typed_ast::Declarator) -> Self {
        match value {
            crate::typed_ast::Declarator::SimpleDeclarator(value) => {
                Declarator::SimpleDeclarator(value.into())
            }
            crate::typed_ast::Declarator::ArrayDeclarator(value) => {
                Declarator::ArrayDeclarator(value.into())
            }
        }
    }
}

impl From<crate::typed_ast::SimpleDeclarator> for SimpleDeclarator {
    fn from(value: crate::typed_ast::SimpleDeclarator) -> Self {
        Self(value.0 .0)
    }
}

impl From<crate::typed_ast::ArrayDeclarator> for ArrayDeclarator {
    fn from(value: crate::typed_ast::ArrayDeclarator) -> Self {
        Self {
            ident: value.ident.0,
            len: value.len.into_iter().map(|v| v.0 .0).collect(),
        }
    }
}
