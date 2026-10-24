//! Lifecycle annotations: site walking, availability intervals and the
//! diagnostics raised while validating them.

mod availability;
mod discovery;
mod intervals;
mod replaces;
mod walk;

pub(crate) use availability::{
    Availability, Site, SiteKind, VersionVisitor, availability, is_versioning_annotation,
};
pub(crate) use discovery::{LifecycleValidator, discover};
pub(crate) use intervals::collect_intervals;
pub(crate) use replaces::{replaces_name, validate as validate_replaces};
