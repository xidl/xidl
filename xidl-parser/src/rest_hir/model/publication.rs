use serde::{Deserialize, Serialize};

use crate::hir::Annotation;

/// Publication of an ordinary HTTP operation in generated API documents.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpOperationPublication {
    /// An operation listed under its HTTP route.
    #[default]
    Path,
    /// A receiver contract listed under its operation identifier.
    Webhook,
}

impl HttpOperationPublication {
    pub(crate) fn from_annotations(interface: &[Annotation], operation: &[Annotation]) -> Self {
        let marked = interface
            .iter()
            .chain(operation)
            .any(|annotation| match annotation {
                Annotation::Builtin { name, .. } => name.eq_ignore_ascii_case("webhooks"),
                Annotation::ScopedName { name, .. } if !name.is_root && name.name.len() == 1 => {
                    name.name[0].eq_ignore_ascii_case("webhooks")
                }
                _ => false,
            });
        if marked { Self::Webhook } else { Self::Path }
    }

    pub(super) fn is_path(&self) -> bool {
        *self == Self::Path
    }
}
