use serde::Serialize;
use std::collections::HashMap;
use xidl_parser::hir;

/// An IDL declaration requiring transport projection.
#[derive(Clone)]
pub enum TransportTypeDef {
    Struct(hir::StructDcl),
    Enum(hir::EnumDcl),
    Typedef(hir::TypedefDcl),
    /// Declaration rendered directly rather than projected into a wire type.
    Public,
}

/// Named declarations available to the transport projector.
pub type TypeRegistry = HashMap<String, TransportTypeDef>;

/// The inbound or outbound wire representation.
#[derive(Clone, Copy)]
pub enum TransportDirection {
    In,
    Out,
}

/// Transport modules rendered for an interface.
#[derive(Serialize)]
pub struct TransportModules {
    pub inbound: TransportModuleContext,
    pub outbound: TransportModuleContext,
}

/// A direction-specific collection of wire types.
#[derive(Serialize)]
pub struct TransportModuleContext {
    pub name: String,
    pub items: Vec<TransportItemContext>,
}

/// A projected struct or enum and its public counterpart.
#[derive(Serialize)]
pub struct TransportItemContext {
    pub kind: String,
    pub transport_ident: String,
    pub public_path: String,
    pub fields: Vec<TransportFieldContext>,
    pub variants: Vec<TransportVariantContext>,
}

/// An enum variant and its wire name.
#[derive(Serialize)]
pub struct TransportVariantContext {
    pub ident: String,
    pub serde_rename: Option<String>,
}

/// A projected field and its conversion expressions.
#[derive(Serialize)]
pub struct TransportFieldContext {
    pub name: String,
    pub ty: String,
    pub serde_rename: Option<String>,
    pub optional: bool,
    pub flatten: bool,
    pub encode_expr: String,
    pub decode_expr: String,
}
