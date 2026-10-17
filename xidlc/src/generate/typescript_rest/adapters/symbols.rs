//! Validate the final module and hook namespaces before rendering.

use super::context::FileContext;
use crate::error::{IdlcError, IdlcResult};
use std::collections::BTreeSet;

impl FileContext {
    pub(super) fn validate_symbols(&self) -> IdlcResult<()> {
        let mut names = self
            .decisions
            .iter()
            .map(|value| value.name.clone())
            .chain(self.conversions.iter().map(|value| value.name.clone()))
            .chain(self.wire_definitions.iter().map(|value| value.name.clone()))
            .chain(self.type_dispatchers.iter().map(|value| value.name.clone()))
            .collect::<Vec<_>>();
        for operation in &self.operations {
            for suffix in ["ResultTo", "ErrorTo"] {
                let prefix = "downcastStep";
                names.push(format!(
                    "{prefix}{}{suffix}{}",
                    operation.prefix, operation.from
                ));
            }
            for suffix in ["ErrorFrom", "ErrorTo"] {
                names.push(format!("{}{suffix}{}", operation.prefix, operation.from));
            }
        }
        for endpoint in &self.endpoints {
            for prefix in ["decodeAndUpcast", "dispatch"] {
                names.push(format!("{prefix}{}From{}", endpoint.prefix, endpoint.from));
            }
            for suffix in ["ResultTo", "ErrorTo", "OutcomeTo"] {
                let prefix = "downcast";
                names.push(format!(
                    "{prefix}{}{suffix}{}",
                    endpoint.prefix, endpoint.from
                ));
            }
        }
        names.extend(
            self.dispatchers
                .iter()
                .map(|value| format!("dispatch{}", value.prefix)),
        );
        let mut symbols = BTreeSet::new();
        for name in &names {
            if !symbols.insert(name) {
                return Err(IdlcError::rpc(format!(
                    "typescript adapters for {}: generated symbol {name} is ambiguous",
                    self.file_stem
                )));
            }
        }
        let mut hooks = BTreeSet::new();
        for hook in &self.hooks {
            if !hooks.insert(&hook.name) {
                return Err(IdlcError::rpc(format!(
                    "typescript adapters for {}: generated hook {} is ambiguous",
                    self.file_stem, hook.name
                )));
            }
        }
        Ok(())
    }
}
