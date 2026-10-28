//! Target-independent types for checked IDL integer expressions.

use super::{BinaryOperator, IntegerType, UnaryOperator};
use serde::{Deserialize, Serialize};

/// The range and signedness of an IDL integer expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegerKind {
    pub(crate) bits: u32,
    pub(crate) signed: bool,
}

impl IntegerKind {
    /// Retains the declared integer width and signedness.
    pub fn from_hir(kind: &IntegerType) -> Self {
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

    /// IDL integer expressions use at least long precision.
    pub fn precision(self) -> u32 {
        self.bits.max(32)
    }

    /// Number of bits in this integer range.
    pub fn bits(self) -> u32 {
        self.bits
    }

    /// Whether the range contains negative integers.
    pub fn is_signed(self) -> bool {
        self.signed
    }

    /// Exact inclusive limits, including the full unsigned long long range.
    pub fn bounds(self) -> (i128, i128) {
        let minimum = if self.signed {
            -(1_i128 << (self.bits - 1))
        } else {
            0
        };
        let maximum = (1_i128 << (self.bits - u32::from(self.signed))) - 1;
        (minimum, maximum)
    }

    /// Bit mask used by IDL complement and zero-filling shifts.
    pub fn mask(self) -> i128 {
        (1_i128 << self.bits) - 1
    }

    pub(crate) fn promoted(self) -> Self {
        Self {
            bits: self.precision(),
            signed: self.signed || self.bits < 32,
        }
    }

    pub(crate) fn with_precision(self, precision: Option<u32>) -> Self {
        let promoted = self.promoted();
        Self {
            bits: precision.unwrap_or(promoted.bits),
            ..promoted
        }
    }

    pub(crate) fn common(self, other: Self) -> Self {
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
}

/// A resolved integer expression graph, without ordinary constant evaluation.
///
/// Steps reference only earlier steps. Repeated constant references share their
/// source steps, preserving declaration boundaries without expanding a tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerExpression {
    /// Steps in dependency order.
    pub steps: Vec<IntegerStep>,
    /// The step containing the constant's assigned value.
    pub result: IntegerStepId,
}

/// An index into the preceding steps of an integer expression graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IntegerStepId(pub usize);

/// An operation and the integer range required for its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerStep {
    /// Range checked by the target compiler.
    pub kind: IntegerKind,
    /// Arithmetic to emit without computing an ordinary constant in xidl.
    pub value: IntegerExpressionValue,
}

/// The finite expression forms supported by IDL integer constants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegerExpressionValue {
    /// An integer literal in decimal notation.
    Literal(String),
    /// A declaration or reference boundary with its own integer range.
    Constant(IntegerStepId),
    /// An IDL unary operation.
    Unary(UnaryOperator, IntegerStepId),
    /// An IDL binary operation.
    Binary(BinaryOperator, IntegerStepId, IntegerStepId),
}
