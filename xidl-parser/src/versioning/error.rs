use std::error::Error;
use std::fmt;

/// Diagnostics raised while analysing or projecting API versioning annotations.
#[derive(Debug, Clone)]
pub enum VersioningError {
    /// `@version` applied to a declaration that is not an enum.
    VersionOutsideEnum { target: String },
    /// Two enums in one specification carry the `@version` marker.
    DuplicateVersionEnum { first: String, second: String },
    /// The `@version` enum declares no members.
    EmptyVersionEnum { ident: String },
    /// Two version members resolve to the same display value.
    DuplicateVersionValue {
        value: String,
        first: String,
        second: String,
    },
    /// A lifecycle annotation appears without a `@version` enum in the specification.
    MissingVersionEnum { annotation: String, decl: String },
    /// A lifecycle annotation argument is not an `Enum::member` reference.
    MalformedVersionReference { annotation: String, decl: String },
    /// `@version` carries arguments; the marker takes none.
    MalformedVersionMarker { decl: String },
    /// `@version(header = "...")` names something that is not an RFC 7230 token.
    MalformedVersionHeader { decl: String },
    /// The `@version` enum declares the version header more than once.
    DuplicateVersionHeader { decl: String },
    /// A lifecycle annotation references a version the timeline does not declare.
    UnknownVersionReference {
        annotation: String,
        decl: String,
        reference: String,
        declared: String,
    },
    /// The projection knows the annotation name but phase 1 does not implement it.
    UnsupportedVersionAnnotation { name: String, decl: String },
    /// A lifecycle annotation applied to a declaration that cannot carry it.
    UnsupportedVersionTarget { annotation: String, target: String },
    /// The `@version` enum itself carries a lifecycle annotation.
    VersionedVersionEnum { annotation: String, ident: String },
    /// The same lifecycle annotation appears twice on one declaration.
    DuplicateLifecycleAnnotation { annotation: String, decl: String },
    /// `@added` and `@removed` leave an empty availability interval.
    EmptyAvailability {
        decl: String,
        added: String,
        removed: String,
    },
    /// A child's annotated interval reaches outside its parent's interval.
    ChildOutsideParent {
        decl: String,
        parent: String,
        version: String,
    },
    /// A surviving declaration references a declaration the projection drops.
    DanglingReference {
        decl: String,
        reference: String,
        version: String,
    },
    /// A version selection was requested for a specification without a `@version` enum.
    UnversionedSpecification { selection: String },
    /// A requested version is not declared by the timeline.
    UnknownSelectedVersion { selection: String, declared: String },
    /// A requested version matches both a member ident and another member's value.
    AmbiguousSelectedVersion {
        selection: String,
        first: String,
        second: String,
    },
    /// The projector received a version position outside the timeline.
    UnknownVersion { index: usize },
    /// Both version selection properties were set.
    ConflictingSelection { first: String, second: String },
    /// The requested version selection is empty.
    EmptySelection,
    /// `api_version` received `*`, which only `api_versions` accepts.
    SelectionRequiresOneVersion { selection: String },
    /// The version layout property holds an unknown value.
    UnknownVersionLayout { value: String },
}

impl fmt::Display for VersioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionOutsideEnum { target } => write!(
                f,
                "xidl versioning: @version is only valid on an enum declaration, not on {target}"
            ),
            Self::DuplicateVersionEnum { first, second } => write!(
                f,
                "xidl versioning: multiple @version enums declared: {first} and {second}"
            ),
            Self::EmptyVersionEnum { ident } => write!(
                f,
                "xidl versioning: the @version enum {ident} declares no versions"
            ),
            Self::DuplicateVersionValue {
                value,
                first,
                second,
            } => write!(
                f,
                "xidl versioning: versions {first} and {second} both resolve to value \"{value}\""
            ),
            Self::MissingVersionEnum { annotation, decl } => write!(
                f,
                "xidl versioning: {annotation} on {decl} requires a @version enum in the specification"
            ),
            Self::MalformedVersionReference { annotation, decl } => write!(
                f,
                "xidl versioning: {annotation} on {decl} requires a version reference like Versions::v1"
            ),
            Self::MalformedVersionMarker { decl } => {
                write!(f, "xidl versioning: @version on {decl} takes no arguments")
            }
            Self::MalformedVersionHeader { decl } => write!(
                f,
                "xidl versioning: @version(header) on {decl} requires a non-empty RFC 7230 token"
            ),
            Self::DuplicateVersionHeader { decl } => write!(
                f,
                "xidl versioning: @version on {decl} declares the version header more than once"
            ),
            Self::UnknownVersionReference {
                annotation,
                decl,
                reference,
                declared,
            } => write!(
                f,
                "xidl versioning: {annotation} on {decl} references {reference}, which is not declared ({declared})"
            ),
            Self::UnsupportedVersionAnnotation { name, decl } => write!(
                f,
                "xidl versioning: {name} on {decl} is not supported yet; phase 1 implements @version, @added and @removed"
            ),
            Self::UnsupportedVersionTarget { annotation, target } => write!(
                f,
                "xidl versioning: {annotation} cannot be applied to {target}"
            ),
            Self::VersionedVersionEnum { annotation, ident } => write!(
                f,
                "xidl versioning: the @version enum {ident} cannot carry {annotation}"
            ),
            Self::DuplicateLifecycleAnnotation { annotation, decl } => {
                write!(f, "xidl versioning: duplicate {annotation} on {decl}")
            }
            Self::EmptyAvailability {
                decl,
                added,
                removed,
            } => write!(
                f,
                "xidl versioning: {decl} is unavailable in every version (@added({added}) and @removed({removed}) leave an empty interval)"
            ),
            Self::ChildOutsideParent {
                decl,
                parent,
                version,
            } => write!(
                f,
                "xidl versioning: {decl} is annotated at {version}, outside the availability of {parent}"
            ),
            Self::DanglingReference {
                decl,
                reference,
                version,
            } => write!(
                f,
                "xidl versioning: {decl} references {reference}, which does not exist in {version}"
            ),
            Self::UnversionedSpecification { selection } => write!(
                f,
                "xidl versioning: version selection {selection} requires a @version enum in the specification"
            ),
            Self::UnknownSelectedVersion {
                selection,
                declared,
            } => write!(
                f,
                "xidl versioning: version {selection} is not declared ({declared})"
            ),
            Self::AmbiguousSelectedVersion {
                selection,
                first,
                second,
            } => write!(
                f,
                "xidl versioning: version {selection} matches both {first} and {second}; use the member ident"
            ),
            Self::UnknownVersion { index } => write!(
                f,
                "xidl versioning: version position {index} is not part of the declared timeline"
            ),
            Self::ConflictingSelection { first, second } => write!(
                f,
                "xidl versioning: {first} and {second} cannot both be set"
            ),
            Self::EmptySelection => write!(
                f,
                "xidl versioning: the version selection is empty; name at least one version or use \"*\""
            ),
            Self::SelectionRequiresOneVersion { selection } => write!(
                f,
                "xidl versioning: {selection} selects one version; use the plural version property with \"*\" for every version"
            ),
            Self::UnknownVersionLayout { value } => write!(
                f,
                "xidl versioning: unknown version layout \"{value}\"; expected \"merged\" or \"per_version\""
            ),
        }
    }
}

impl Error for VersioningError {}
