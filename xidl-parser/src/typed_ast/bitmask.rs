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
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct BitsetDcl {
    pub ident: Identifier,
    pub parent: Option<ScopedName>,
    #[ts(id = "bitfield")]
    pub field: Vec<BitField>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
#[ts(id = "bitfield")]
pub struct BitField {
    pub spec: BitfieldSpec,
    pub ident: Vec<Identifier>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct BitfieldSpec {
    pub pos: PositiveIntConst,
    pub dst_ty: Option<DestinationType>,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub enum DestinationType {
    BooleanType(BooleanType),
    OctetType(OctetType),
    IntegerType(IntegerType),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct BitmaskDcl {
    pub ident: Identifier,

    pub value: Vec<BitValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitValue {
    pub annotations: Vec<AnnotationAppl>,
    pub ident: Identifier,
}

impl<'a> crate::parser::FromTreeSitter<'a> for BitValue {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        assert_eq!(node.kind_id(), xidl_parser_derive::node_id!("bit_value"));
        let mut annotations = Vec::new();
        let mut ident = None;
        for ch in node.children(&mut node.walk()) {
            match ch.kind_id() {
                xidl_parser_derive::node_id!("annotation_appl")
                | xidl_parser_derive::node_id!("extend_annotation_appl") => {
                    annotations.push(AnnotationAppl::from_node(ch, ctx)?);
                }
                xidl_parser_derive::node_id!("identifier") => {
                    ident = Some(Identifier::from_node(ch, ctx)?);
                }
                _ => {}
            }
        }
        if let Some(doc) = ctx.take_doc_comment(&node) {
            annotations.insert(0, AnnotationAppl::doc(doc));
        }
        Ok(Self {
            annotations,
            ident: ident.ok_or_else(|| {
                crate::error::ParseError::UnexpectedNode(format!(
                    "parent: {}, got: missing identifier",
                    node.kind()
                ))
            })?,
        })
    }
}
