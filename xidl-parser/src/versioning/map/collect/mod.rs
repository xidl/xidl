//! Collects shallow type and HTTP facts from projected HIR.

mod http;
mod render;
mod types;

pub(super) use http::collect_operations;
pub(super) use types::collect_types;
