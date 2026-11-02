use super::Site;
use crate::contract::Contract;
use crate::error::{DocumentationError, Result};
use crate::markdown::Markdown;
use crate::notes::ReleaseNote;
use std::collections::BTreeSet;
use xidlc::jsonrpc::CompilationInput;

impl Site<'_> {
    pub(super) fn validate_notes(
        &self,
        notes: &[ReleaseNote],
        compilation: &CompilationInput,
        contracts: &[Contract],
    ) -> Result<()> {
        let mut identities = BTreeSet::new();
        for note in notes {
            if !identities.insert((&note.unit, &note.from, &note.to)) {
                return Err(DocumentationError::Contract(format!(
                    "duplicate release notes for '{}': {} to {}",
                    note.unit, note.from, note.to
                )));
            }
            let known = compilation
                .units
                .iter()
                .filter(|unit| unit.id == note.unit)
                .filter_map(|unit| unit.version_map.as_ref())
                .any(|map| {
                    map.steps
                        .iter()
                        .any(|step| step.from == note.from && step.to == note.to)
                });
            if !known {
                return Err(DocumentationError::Contract(format!(
                    "release note '{}' names an unknown adjacent version pair",
                    note.title
                )));
            }
            let selected = [note.from.as_str(), note.to.as_str()]
                .iter()
                .all(|version| {
                    contracts
                        .iter()
                        .any(|contract| contract.unit == note.unit && contract.version == *version)
                });
            let published = contracts.iter().any(|contract| {
                contract.unit == note.unit
                    && [note.from.as_str(), note.to.as_str()].contains(&contract.version.as_str())
                    && !contract.operations.is_empty()
            });
            let root = compilation
                .units
                .iter()
                .any(|unit| unit.id == note.unit && unit.is_root);
            if !selected || !root || !published || self.config.excluded(&note.unit, "") {
                continue;
            }
            let routes = contracts
                .iter()
                .filter(|contract| {
                    contract.unit == note.unit
                        && (contract.version == note.from || contract.version == note.to)
                })
                .flat_map(|contract| {
                    contract
                        .operations
                        .values()
                        .flat_map(|operation| operation.routes.clone())
                })
                .collect::<Vec<_>>();
            for content in [&note.markdown, &note.impact, &note.upgrade] {
                Markdown::validate_examples(content, &routes)?;
            }
        }
        Ok(())
    }
}
