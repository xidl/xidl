mod attr;
mod model;
mod project;
mod semantics;
#[cfg(test)]
mod tests;

pub use model::*;
pub use project::{collect_diagnostics, project};
