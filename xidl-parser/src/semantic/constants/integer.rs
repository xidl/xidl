//! IDL integer precision and checked operations.
//! See OMG IDL 4.2 section 7.4.1.4.3: https://www.omg.org/spec/IDL/4.2/PDF.

use crate::error::{ParseError, ParserResult};
use crate::hir::{BinaryOperator, UnaryOperator};

pub(super) use crate::hir::IntegerKind;

impl IntegerKind {
    fn check(self, value: i128) -> ParserResult<Integer> {
        let (minimum, maximum) = self.bounds();
        if !(minimum..=maximum).contains(&value) {
            return Err(ParseError::Message(format!(
                "integer constant value {value} exceeds {}-bit {} integer range",
                self.bits,
                if self.signed { "signed" } else { "unsigned" }
            )));
        }
        Ok(Integer { value, kind: self })
    }

    fn decode(self, value: i128) -> ParserResult<Integer> {
        let value = value & self.mask();
        let value = if self.signed && value & (1_i128 << (self.bits - 1)) != 0 {
            value - (1_i128 << self.bits)
        } else {
            value
        };
        self.check(value)
    }
}

/// An integer expression value, including unsigned 64-bit intermediates.
#[derive(Clone, Copy)]
pub(super) struct Integer {
    value: i128,
    kind: IntegerKind,
}

impl Integer {
    /// Integer literals default to unsigned long or unsigned long long.
    pub(super) fn literal(value: &str, precision: Option<u32>) -> ParserResult<Self> {
        let value = value.parse::<u64>().map_err(|_| {
            ParseError::Message(format!(
                "integer constant literal '{value}' is not an integer"
            ))
        })?;
        let bits = precision.unwrap_or(if value > u64::from(u32::MAX) { 64 } else { 32 });
        IntegerKind {
            bits,
            signed: false,
        }
        .check(i128::from(value))
    }

    /// Uses the enclosing constant's precision for its subexpressions.
    pub(super) fn with_precision(mut self, precision: Option<u32>) -> ParserResult<Self> {
        self.kind = self.kind.with_precision(precision);
        self.kind.check(self.value)
    }

    /// Checks the final value against the constant's declared range.
    pub(super) fn assign(self, kind: IntegerKind) -> ParserResult<Self> {
        kind.check(self.value)
    }

    /// Applies IDL unary arithmetic and width-aware complement.
    pub(super) fn unary(self, operator: &UnaryOperator) -> ParserResult<Self> {
        let kind = self.kind.promoted();
        match operator {
            UnaryOperator::Add => Ok(self),
            UnaryOperator::Sub => IntegerKind {
                signed: true,
                ..kind
            }
            .check(-self.value),
            UnaryOperator::Not => kind.decode(!self.value),
        }
    }

    /// Evaluates IDL arithmetic; shifts always fill vacated bits with zero.
    pub(super) fn binary(self, operator: BinaryOperator, right: Self) -> ParserResult<Self> {
        let kind = self.kind.common(right.kind);
        let left = self.value;
        let right = right.value;
        let shift = u32::try_from(right).ok().filter(|shift| *shift < 64);
        match operator {
            BinaryOperator::Or => return kind.decode(left | right),
            BinaryOperator::Xor => return kind.decode(left ^ right),
            BinaryOperator::And => return kind.decode(left & right),
            BinaryOperator::RightShift => {
                if let Some(shift) = shift {
                    return kind.decode((left & kind.mask()) >> shift);
                }
            }
            _ => {}
        }
        let value = match operator {
            BinaryOperator::LeftShift => shift.and_then(|shift| left.checked_mul(1_i128 << shift)),
            BinaryOperator::Add => left.checked_add(right),
            BinaryOperator::Sub => left.checked_sub(right),
            BinaryOperator::Mult => left.checked_mul(right),
            BinaryOperator::Div => left.checked_div(right),
            BinaryOperator::Mod => left.checked_rem(right),
            _ => None,
        }
        .ok_or_else(|| {
            ParseError::Message(format!(
                "integer constant operation '{left} {operator:?} {right}' is invalid or overflows"
            ))
        })?;
        kind.check(value)
    }

    /// Exposes the literal type without evaluating any arithmetic expression.
    pub(super) fn kind(self) -> IntegerKind {
        self.kind
    }

    /// Keeps normalized dimensions readable by every existing HIR consumer.
    pub(super) fn dimension(self) -> ParserResult<i64> {
        i64::try_from(self.value).map_err(|_| {
            ParseError::Message(format!(
                "array size {} exceeds the supported dimension range",
                self.value
            ))
        })
    }
}
