//! ```js
//! exports.rules = {
//!   bitset_dcl: $ =>
//!     seq(
//!       'bitset',
//!       $.identifier,
//!       optional(seq(':', $.scoped_name)),
//!       '{',
//!       repeat($.bitfield),
//!       '}',
//!     ),
//!   bitfield: $ => seq($.bitfield_spec, repeat($.identifier), ';'),
//!   bitfield_spec: $ =>
//!     seq(
//!       'bitfield',
//!       '<',
//!       $.positive_int_const,
//!       optional(seq(',', $.destination_type)),
//!       '>',
//!     ),
//!   destination_type: $ => choice($.boolean_type, $.octet_type, $.integer_type),
//!
//!   bitmask_dcl: $ =>
//!     seq(
//!       'bitmask',
//!       $.identifier,
//!       '{',
//!       commaSep($.bit_value),
//!       optional(','),
//!       '}',
//!     ),
//!   bit_value: $ => seq(repeat($.annotation_appl), $.identifier),
//! }
//! ```

use super::*;

#[derive(Debug, Parser)]
pub struct BitsetDcl {
    pub ident: Identifier,
    pub parent: Option<ScopedName>,
    #[ts(id = "bitfield")]
    pub field: Vec<BitField>,
}

#[derive(Debug, Parser)]
#[ts(id = "bitfield")]
pub struct BitField {
    pub spec: BitfieldSpec,
    pub ident: Vec<Identifier>,
}

#[derive(Debug, Parser)]
pub struct BitfieldSpec {
    pub pos: PositiveIntConst,
    pub dst_ty: Option<DestinationType>,
}

#[derive(Debug, Parser)]
pub enum DestinationType {
    BooleanType(BooleanType),
    OctetType(OctetType),
    IntegerType(IntegerType),
}

#[derive(Debug, Parser)]
pub struct BitmaskDcl {
    pub ident: Identifier,

    pub value: Vec<BitValue>,
}

#[derive(Debug, Parser)]
pub struct BitValue(pub Vec<Identifier>);
