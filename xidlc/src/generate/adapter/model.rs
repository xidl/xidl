//! Typed surfaces and decisions shared by adapter renderers.

use crate::generate::adapter::Updates;
use std::collections::BTreeMap;
use xidl_parser::hir;
use xidl_parser::rest_hir::HttpOperation;
use xidl_parser::versioning::map::MapVersion;

#[derive(Debug, Clone)]
pub(crate) struct Member {
    pub name: String,
    pub wire: String,
    pub flatten: bool,
    pub key: String,
    /// Canonical base declarations from the immediate base to the field owner.
    /// An empty path identifies a field declared on the current type.
    pub path: Vec<String>,
    pub value: Option<TypeUse>,
}

/// Runtime value representation at a model or HTTP binding site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueRepresentation {
    Model,
    ByteBuffer,
}

#[derive(Debug, Clone)]
pub(crate) struct TypeUse {
    /// Root names are absolute within this version surface, without its wrapper.
    pub ty: hir::TypeSpec,
    pub scope: Vec<String>,
    pub declarator: hir::Declarator,
    pub optional: bool,
    /// The declaration states `@default`, so an older version's implied value
    /// is a required upcast decision instead of a silent absence.
    pub defaulted: bool,
    pub representation: ValueRepresentation,
}

#[derive(Debug, Clone)]
pub(crate) enum TypeShape {
    Struct(Vec<Member>),
    Enum(Vec<Member>),
    Union(Vec<Member>),
    Alias(TypeUse),
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub(crate) struct Operation {
    pub interface: String,
    pub name: String,
    pub scope: Vec<String>,
    pub inputs: Vec<Member>,
    pub outputs: Vec<Member>,
    pub raises: Vec<String>,
    pub http: HttpOperation,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct VersionSurface {
    pub types: BTreeMap<String, TypeShape>,
    pub operations: BTreeMap<String, Operation>,
}

/// A value conversion preserves container structure and short-circuits errors.
#[derive(Debug, Clone)]
pub(crate) enum ValuePlan {
    Identity,
    Named(String),
    Sequence(Box<Self>),
    Map { key: Box<Self>, value: Box<Self> },
    Optional(Box<Self>),
    Array(Box<Self>),
}

#[derive(Debug, Clone)]
pub(crate) struct FieldCopy {
    pub source: Member,
    pub target: Member,
    pub value: ValuePlan,
}

/// Decisions keep target assignments separate from ignored source values.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConversionPlan {
    pub copies: Vec<FieldCopy>,
    pub assignments: Vec<Member>,
    pub ignored: Vec<Member>,
    pub absences: Vec<Member>,
    pub consumed: Vec<Member>,
}

#[derive(Debug, Clone)]
pub(crate) enum TypeConversion {
    Hook,
    Struct(ConversionPlan),
    Enum(ConversionPlan),
    Union(ConversionPlan),
    Alias(ValuePlan),
}

#[derive(Debug, Clone)]
pub(crate) struct TypePlan {
    pub key: String,
    pub upcast: Option<TypeConversion>,
    pub downcast: Option<TypeConversion>,
}

#[derive(Debug, Clone)]
pub(crate) struct OperationPlan {
    pub key: String,
    pub source: Operation,
    pub target: Operation,
    pub upcast: ConversionPlan,
    pub downcast: ConversionPlan,
}

#[derive(Debug, Clone)]
pub(crate) struct StepPlan {
    pub from: MapVersion,
    pub to: MapVersion,
    pub source_types: BTreeMap<String, TypeShape>,
    pub target_types: BTreeMap<String, TypeShape>,
    pub types: Vec<TypePlan>,
    pub operations: Vec<OperationPlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct AdapterPlan {
    pub versions: Vec<VersionPlan>,
    pub steps: Vec<StepPlan>,
    /// Per type or operation key, the wire names each served version still
    /// reads from storage, newest version first; Head maps to an empty list.
    /// Derived from `@added` / `@removed` alone: a version reads every member
    /// removed by the adjacent steps at or after its own position.
    pub needs: BTreeMap<String, Vec<(String, Vec<String>)>>,
    /// Update operations whose request upcast receives the Head resource and
    /// hands paired retired members back to the handler.
    pub updates: Updates,
}

/// Complete surfaces also preserve Head and contracts with a single version.
#[derive(Debug, Clone)]
pub(crate) struct VersionPlan {
    pub version: MapVersion,
    pub surface: VersionSurface,
}
