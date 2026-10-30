/// Declaration category used by annotation completion and placement checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// An IDL module.
    Module,
    /// A struct declaration.
    Struct,
    /// A union declaration.
    Union,
    /// An enum declaration.
    Enum,
    /// A bitset declaration.
    Bitset,
    /// A bitmask declaration.
    Bitmask,
    /// A typedef declaration.
    Typedef,
    /// A native declaration.
    Native,
    /// A constant declaration.
    Constant,
    /// An exception declaration.
    Exception,
    /// A struct or exception member.
    Member,
    /// A union branch member.
    UnionMember,
    /// An enum value.
    Enumerator,
    /// A bitmask value.
    BitmaskValue,
    /// An interface declaration.
    Interface,
    /// An operation declaration.
    Operation,
    /// An operation parameter.
    Parameter,
    /// An interface attribute.
    Attribute,
}

/// Syntactic argument contract, independent of generation and application policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentShape {
    /// No arguments.
    Marker,
    /// A value, optionally named `value`.
    Value,
    /// An optional value, optionally named `value`.
    OptionalValue,
    /// One string, optionally under one of these names.
    String(&'static [&'static str]),
    /// An optional string, optionally under one of these names.
    OptionalString(&'static [&'static str]),
    /// A single scoped version member reference.
    Lifecycle,
    /// A version marker, or `in` and `name` together.
    Version,
    /// Required `in` and `name` arguments.
    ApiKey,
    /// HTTP status on exceptions, or a marker on representation unions.
    Http,
    /// Named arguments with the specified required and optional keys.
    Named {
        /// Keys required by this annotation.
        required: &'static [&'static str],
        /// Additional accepted keys.
        optional: &'static [&'static str],
    },
    /// One or more positional values, as accepted by target-specific derives.
    Values,
    /// Origins accepted by the HTTP CORS contract.
    Cors,
    /// A deprecation marker with optional since and after metadata.
    Deprecated,
    /// Arguments already validated by the concrete IDL grammar.
    Grammar,
}

/// A built-in annotation's public completion and validation metadata.
#[derive(Debug, Clone, Copy)]
pub struct AnnotationDefinition {
    /// Canonical ASCII name; matching ignores ASCII case.
    pub name: &'static str,
    /// Typed identity for semantic consumers.
    pub kind: AnnotationKind,
    /// Allowed declaration sites; empty means every annotation-bearing site.
    pub sites: &'static [Site],
    /// Accepted argument shape.
    pub arguments: ArgumentShape,
}

impl AnnotationDefinition {
    /// Looks up a built-in name using the shared ASCII case rule.
    pub fn lookup(name: &str) -> Option<&'static Self> {
        BUILTIN_ANNOTATIONS
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }

    /// Whether this definition can annotate the given declaration.
    pub fn allows(&self, site: Site) -> bool {
        self.sites.is_empty() || self.sites.contains(&site)
    }

    /// Whether the name is a supported target-specific pass-through attribute.
    pub fn is_passthrough(name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        ["rust-", "rust_", "ts-", "ts_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
    }
}

macro_rules! annotations {
    ($($kind:ident, $name:literal, $sites:expr, $args:expr;)+) => {
        /// Typed identity of an annotation from the shared built-in table.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum AnnotationKind {
            $(#[doc = concat!("The `@", $name, "` annotation.")] $kind,)+
        }
        /// Built-in annotations shared by lint, semantic consumers and editors.
        pub const BUILTIN_ANNOTATIONS: &[AnnotationDefinition] = &[
            $(AnnotationDefinition { name: $name, kind: AnnotationKind::$kind,
                sites: $sites, arguments: $args },)+
        ];
    };
}

use ArgumentShape::*;
use Site::*;

const TYPES: &[Site] = &[
    Struct, Union, Enum, Bitset, Bitmask, Typedef, Native, Exception,
];
const FIELDS: &[Site] = &[Member, UnionMember, Parameter];
const SECURITY: &[Site] = &[Interface, Operation, Attribute];
const DEPRECATABLE: &[Site] = &[
    Module, Struct, Union, Enum, Exception, Member, Enumerator, Interface, Operation, Parameter,
    Attribute,
];
const LIFECYCLE: &[Site] = &[
    Module,
    Struct,
    Union,
    Enum,
    Bitset,
    Bitmask,
    Typedef,
    Native,
    Exception,
    Member,
    UnionMember,
    Enumerator,
    BitmaskValue,
    Interface,
    Operation,
    Parameter,
    Attribute,
];

annotations! {
    Id, "id", &[Member, UnionMember], Grammar;
    AutoId, "autoid", &[Module, Struct, Union, Exception], Grammar;
    Optional, "optional", FIELDS, OptionalValue;
    Position, "position", &[BitmaskValue], Grammar;
    Value, "value", &[Enumerator], Grammar;
    Extensibility, "extensibility", TYPES, Grammar;
    Final, "final", TYPES, Marker;
    Appendable, "appendable", TYPES, Marker;
    Mutable, "mutable", TYPES, Marker;
    Key, "key", &[Member, UnionMember], OptionalValue;
    MustUnderstand, "must_understand", &[Member, UnionMember], OptionalValue;
    DefaultLiteral, "default_literal", &[Enumerator], Marker;
    Default, "default", FIELDS, Value;
    Range, "range", FIELDS, Named { required: &["min", "max"], optional: &[] };
    Min, "min", FIELDS, Value;
    Max, "max", FIELDS, Value;
    Unit, "unit", FIELDS, Value;
    BitBound, "bit_bound", &[Enum, Bitmask], Value;
    External, "external", FIELDS, OptionalValue;
    Nested, "nested", TYPES, OptionalValue;
    Verbatim, "verbatim", &[], Grammar;
    Service, "service", &[Interface], Grammar;
    Oneway, "oneway", &[Operation], OptionalValue;
    Webhooks, "webhooks", &[Interface, Operation], Marker;
    Ami, "ami", &[Interface, Operation], OptionalValue;
    HashId, "hashid", &[Member, UnionMember], OptionalValue;
    DefaultNested, "default_nested", &[Module], OptionalValue;
    IgnoreLiteralNames, "ignore_literal_names", &[Enum], OptionalValue;
    TryConstruct, "try_construct", FIELDS, Grammar;
    NonSerialized, "non_serialized", FIELDS, OptionalValue;
    DataRepresentation, "data_representation", TYPES, Grammar;
    Topic, "topic", &[Struct, Union], Grammar;
    Choice, "choice", &[Union], Marker;
    Empty, "empty", &[Struct], Marker;
    Rename, "rename", &[], String(&["name"]);
    Name, "name", &[], String(&["name"]);
    RenameAll, "rename_all", &[Struct, Union, Enum, Bitmask, Exception, Interface], String(&["rule"]);
    Skip, "skip", &[Member, UnionMember, Enumerator, BitmaskValue, Parameter], Marker;
    OmitEmpty, "omitempty", &[Member], Marker;
    Doc, "doc", &[], String(&["text", "doc"]);
    Derive, "derive", TYPES, Values;
    Rust, "rust", &[], Value;
    Version, "version", &[Enum], Version;
    Added, "added", LIFECYCLE, Lifecycle;
    Removed, "removed", LIFECYCLE, Lifecycle;
    Http, "http", &[Exception, Union], Http;
    Get, "get", &[Operation], OptionalString(&["path"]);
    Post, "post", &[Operation], OptionalString(&["path"]);
    Put, "put", &[Operation], OptionalString(&["path"]);
    Patch, "patch", &[Operation], OptionalString(&["path"]);
    Delete, "delete", &[Operation], OptionalString(&["path"]);
    Head, "head", &[Operation], OptionalString(&["path"]);
    Options, "options", &[Operation], OptionalString(&["path"]);
    Path, "path", &[Interface, Operation, Parameter], OptionalString(&["name", "path"]);
    Query, "query", &[Parameter], OptionalString(&["name"]);
    Header, "header", &[Parameter, Member], OptionalString(&["name"]);
    Cookie, "cookie", &[Parameter, Member], OptionalString(&["name"]);
    Body, "body", &[Parameter, Member], OptionalString(&[]);
    Flatten, "flatten", &[Member, Parameter], Marker;
    HttpBasic, "http_basic", SECURITY, OptionalString(&["realm"]);
    HttpBearer, "http_bearer", SECURITY, Marker;
    ApiKey, "api_key", SECURITY, ApiKey;
    NoSecurity, "no_security", SECURITY, Marker;
    Request, "request", &[Interface, Operation], String(&["format"]);
    Response, "response", &[Interface, Operation], String(&["format"]);
    ServerStream, "server_stream", &[Operation, Attribute], Marker;
    ClientStream, "client_stream", &[Operation], Marker;
    BidiStream, "bidi_stream", &[Operation], Marker;
    StreamCodec, "stream_codec", &[Operation], OptionalString(&[]);
    Upgrade, "upgrade", &[Operation], Named { required: &["protocol"], optional: &["codec", "subprotocol", "heartbeat", "max_message"] };
    Deprecated, "deprecated", DEPRECATABLE, Deprecated;
    Cors, "cors", &[Interface, Operation], Cors;
}

impl AnnotationKind {
    /// Recognizes a name without retaining a second spelling table.
    pub fn from_name(name: &str) -> Option<Self> {
        AnnotationDefinition::lookup(name).map(|entry| entry.kind)
    }

    /// Whether projection removes this annotation after evaluating the timeline.
    pub fn is_lifecycle(self) -> bool {
        matches!(self, Self::Version | Self::Added | Self::Removed)
    }
}
