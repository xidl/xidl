//! Parse expression alternatives from the concrete grammar's child nodes.

use super::*;
use crate::error::{ParseError, ParserResult};
use crate::parser::{FromTreeSitter, ParseContext};
use tree_sitter::Node;

impl<'a> ParseContext<'a> {
    fn expression_children(&self, node: Node<'a>, expected: &str) -> ParserResult<Vec<Node<'a>>> {
        if node.kind() != expected {
            return Err(ParseError::UnexpectedNode(format!(
                "IDL expression expected {expected}, got {}",
                node.kind()
            )));
        }
        Ok(node
            .children(&mut node.walk())
            .filter(|child| !child.is_extra())
            .collect())
    }

    fn invalid_expression(&self, node: Node<'a>) -> ParseError {
        ParseError::UnexpectedNode(format!(
            "IDL expression has invalid {} children",
            node.kind()
        ))
    }
}

impl<'a> FromTreeSitter<'a> for ConstExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "const_expr")?.as_slice() {
            [value] => Ok(Self(OrExpr::from_node(*value, ctx)?)),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for OrExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "or_expr")?.as_slice() {
            [value] => Ok(Self::XorExpr(XorExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "|" => Ok(Self::OrExpr(
                Box::new(Self::from_node(*left, ctx)?),
                XorExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for XorExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "xor_expr")?.as_slice() {
            [value] => Ok(Self::AndExpr(AndExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "^" => Ok(Self::XorExpr(
                Box::new(Self::from_node(*left, ctx)?),
                AndExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for AndExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "and_expr")?.as_slice() {
            [value] => Ok(Self::ShiftExpr(ShiftExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "&" => Ok(Self::AndExpr(
                Box::new(Self::from_node(*left, ctx)?),
                ShiftExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for ShiftExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "shift_expr")?.as_slice() {
            [value] => Ok(Self::AddExpr(AddExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "<<" => Ok(Self::LeftShiftExpr(
                Box::new(Self::from_node(*left, ctx)?),
                AddExpr::from_node(*right, ctx)?,
            )),
            [left, operator, right] if operator.kind() == ">>" => Ok(Self::RightShiftExpr(
                Box::new(Self::from_node(*left, ctx)?),
                AddExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for AddExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "add_expr")?.as_slice() {
            [value] => Ok(Self::MultExpr(MultExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "+" => Ok(Self::AddExpr(
                Box::new(Self::from_node(*left, ctx)?),
                MultExpr::from_node(*right, ctx)?,
            )),
            [left, operator, right] if operator.kind() == "-" => Ok(Self::SubExpr(
                Box::new(Self::from_node(*left, ctx)?),
                MultExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for MultExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "mult_expr")?.as_slice() {
            [value] => Ok(Self::UnaryExpr(UnaryExpr::from_node(*value, ctx)?)),
            [left, operator, right] if operator.kind() == "*" => Ok(Self::MultExpr(
                Box::new(Self::from_node(*left, ctx)?),
                UnaryExpr::from_node(*right, ctx)?,
            )),
            [left, operator, right] if operator.kind() == "/" => Ok(Self::DivExpr(
                Box::new(Self::from_node(*left, ctx)?),
                UnaryExpr::from_node(*right, ctx)?,
            )),
            [left, operator, right] if operator.kind() == "%" => Ok(Self::ModExpr(
                Box::new(Self::from_node(*left, ctx)?),
                UnaryExpr::from_node(*right, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for UnaryExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "unary_expr")?.as_slice() {
            [value] => Ok(Self::PrimaryExpr(PrimaryExpr::from_node(*value, ctx)?)),
            [operator, value] => Ok(Self::UnaryExpr(
                UnaryOperator::from_node(*operator, ctx)?,
                PrimaryExpr::from_node(*value, ctx)?,
            )),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for PrimaryExpr {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "primary_expr")?.as_slice() {
            [value] if value.kind() == "scoped_name" => {
                Ok(Self::ScopedName(ScopedName::from_node(*value, ctx)?))
            }
            [value] if value.kind() == "literal" => {
                Ok(Self::Literal(Literal::from_node(*value, ctx)?))
            }
            [open, value, close] if open.kind() == "(" && close.kind() == ")" => Ok(
                Self::ConstExpr(Box::new(ConstExpr::from_node(*value, ctx)?)),
            ),
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}

impl<'a> FromTreeSitter<'a> for UnaryOperator {
    fn from_node(node: Node<'a>, ctx: &mut ParseContext<'a>) -> ParserResult<Self> {
        match ctx.expression_children(node, "unary_operator")?.as_slice() {
            [operator] => match operator.kind() {
                "+" => Ok(Self::Add),
                "-" => Ok(Self::Sub),
                "~" => Ok(Self::Not),
                _ => Err(ctx.invalid_expression(node)),
            },
            _ => Err(ctx.invalid_expression(node)),
        }
    }
}
