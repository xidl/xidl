//! Discriminator spellings shared by generated union types, codecs and schemas.

use xidl_parser::hir;

/// The ordinary tagged-union JSON layout, independent of a target language.
pub(crate) struct UnionWire;

impl UnionWire {
    /// Wire tags selecting a case; a default case has the explicit tag `default`.
    pub(crate) fn tags(case: &hir::Case) -> Vec<String> {
        case.label.iter().map(Self::label).collect()
    }

    /// The stable discriminator of a source case label.
    pub(crate) fn label(label: &hir::CaseLabel) -> String {
        match label {
            hir::CaseLabel::Default => "default".into(),
            hir::CaseLabel::Value(value) => crate::generate::render_const_expr(
                value,
                &|name| name.name.last().cloned().unwrap_or_default(),
                &Self::literal,
                &|operator| match operator {
                    hir::UnaryOperator::Add => "",
                    hir::UnaryOperator::Sub => "-",
                    hir::UnaryOperator::Not => "!",
                },
            ),
        }
    }

    fn literal(literal: &hir::Literal) -> String {
        match literal {
            hir::Literal::IntegerLiteral(value) => value.0.clone(),
            hir::Literal::FloatingPtLiteral(value) => format!(
                "{}{}.{}",
                value
                    .sign
                    .as_ref()
                    .map(hir::IntegerSign::as_str)
                    .unwrap_or(""),
                value.integer.0,
                value.fraction.0
            ),
            hir::Literal::CharLiteral(value) | hir::Literal::StringLiteral(value) => value.clone(),
            hir::Literal::WideCharacterLiteral(value) | hir::Literal::WideStringLiteral(value) => {
                value.strip_prefix('L').unwrap_or(value).into()
            }
            hir::Literal::BooleanLiteral(value) => value.to_string(),
        }
    }
}
