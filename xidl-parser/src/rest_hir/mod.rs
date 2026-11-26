mod attr;
mod exceptions;
pub mod mapping;
mod model;
mod project;
pub(crate) mod project_params;
mod route;
pub mod semantics;
mod stream_errors;
#[cfg(test)]
mod tests;
mod types;
mod unions;
mod validate;

pub use crate::pipeline::{HirProjectionKind, ProjectedHir};
pub use mapping::is_byte_sequence;
pub use model::ProjectOptions;
pub use model::*;
pub use project::{project, project_with_options};
pub use semantics::{UpgradeMode, WebSocketCodec, WebSocketConfig};

pub(crate) type RestHirResult<T> = Result<T, String>;
