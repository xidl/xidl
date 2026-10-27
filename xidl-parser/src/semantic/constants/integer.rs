//! IDL integer precision and checked operations.
//! See OMG IDL 4.2 section 7.4.1.4.3: https://www.omg.org/spec/IDL/4.2/PDF.

use crate::error::{ParseError, ParserResult};
use crate::hir::{BinaryOperator, ConstExpr, IntegerLiteral, IntegerType, Literal, UnaryOperator};

/// The declared range of an IDL integer constant.
#[derive(Clone, Copy)]
pub(super) struct IntegerKind {
    bits: u32,
    signed: bool,
}

impl IntegerKind {
    /// Retains signedness and width without depending on a target language.
    pub(super) fn from_hir(kind: &IntegerType) -> Self {
        let (bits, signed) = match kind {
            IntegerType::I8 => (8, true),
            IntegerType::I16 => (16, true),
            IntegerType::I32 => (32, true),
            IntegerType::I64 => (64, true),
            IntegerType::Char | IntegerType::UChar | IntegerType::Octet | IntegerType::U8 => {
                (8, false)
            }
            IntegerType::U16 => (16, false),
            IntegerType::U32 => (32, false),
            IntegerType::U64 => (64, false),
        };
        Self { bits, signed }
    }

    /// IDL constant expressions use at least long precision.
    pub(super) fn precision(self) -> u32 {
        self.bits.max(32)
    }

    fn check(self, value: i128) -> ParserResult<Integer> {
        let minimum = if self.signed {
            -(1_i128 << (self.bits - 1))
        } else {
            0
        };
        let maximum = (1_i128 << (self.bits - u32::from(self.signed))) - 1;
        if !(minimum..=maximum).contains(&value) {
            return Err(ParseError::Message(format!(
                "integer constant value {value} exceeds {}-bit {} integer range",
                self.bits,
                if self.signed { "signed" } else { "unsigned" }
            )));
        }
        Ok(Integer { value, kind: self })
    }

    fn promoted(self) -> Self {
        Self {
            bits: self.precision(),
            signed: self.signed || self.bits < 32,
        }
    }

    fn common(self, other: Self) -> Self {
        let left = self.promoted();
        let right = other.promoted();
        let bits = left.bits.max(right.bits);
        let unsigned = [left, right]
            .iter()
            .any(|kind| kind.bits == bits && !kind.signed);
        Self {
            bits,
            signed: !unsigned,
        }
    }

    fn mask(self) -> i128 {
        (1_i128 << self.bits) - 1
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
        self.kind = self.kind.promoted();
        if let Some(bits) = precision {
            self.kind.bits = bits;
        }
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

    /// Retains the HIR distinction between a positive literal and unary minus.
    pub(super) fn expression(self) -> ConstExpr {
        let literal = ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral(
            self.value.abs().to_string(),
        )));
        if self.value < 0 {
            ConstExpr::UnaryExpr(UnaryOperator::Sub, Box::new(literal))
        } else {
            literal
        }
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
