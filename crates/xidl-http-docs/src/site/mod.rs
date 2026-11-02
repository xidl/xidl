mod indexes;
mod notes;
mod output;
mod playgrounds;
mod references;
mod starlight;

#[cfg(test)]
mod tests;

use crate::changes::Changes;
use crate::config::DocumentationConfig;
use crate::contract::Contract;
use crate::error::{DocumentationError, Result};
use crate::links::{Links, SchemaId};
use crate::notes::ReleaseNote;
use crate::page::{Page, PageLink};
use crate::render::Renderer;
use crate::schema::SchemaView;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use xidlc::jsonrpc::{Artifact, COMPILATION_FORMAT, CompilationInput};

pub(crate) struct Site<'a> {
    config: &'a DocumentationConfig,
    renderer: Renderer,
    pages: BTreeMap<String, Page>,
    schemas: BTreeMap<SchemaId, Value>,
    uses: BTreeMap<SchemaId, Vec<PageLink>>,
}

impl<'a> Site<'a> {
    pub fn build(
        mut compilation: CompilationInput,
        config: &'a DocumentationConfig,
    ) -> Result<Vec<Artifact>> {
        if compilation.format != COMPILATION_FORMAT {
            return Err(DocumentationError::Contract(format!(
                "unsupported compilation format {}; expected {COMPILATION_FORMAT}",
                compilation.format
            )));
        }
        let mut site = Self {
            config,
            renderer: Renderer::new()?,
            pages: BTreeMap::new(),
            schemas: BTreeMap::new(),
            uses: BTreeMap::new(),
        };
        compilation.units.sort_by(|a, b| a.id.cmp(&b.id));
        let mut contracts = Vec::new();
        let mut identities = BTreeSet::new();
        for unit in &compilation.units {
            if !identities.insert(unit.id.clone()) {
                return Err(DocumentationError::Contract(format!(
                    "duplicate compilation unit '{}'",
                    unit.id
                )));
            }
            if config.excluded(&unit.id, "") {
                continue;
            }
            for input in &unit.inputs {
                contracts.push(Contract::from_input(unit, input, config)?);
            }
        }
        site.collect_schemas(&contracts)?;
        site.references(&contracts)?;
        let notes = config
            .notes
            .iter()
            .map(|path| ReleaseNote::read(path))
            .collect::<Result<Vec<_>>>()?;
        site.validate_notes(&notes, &compilation, &contracts)?;
        for unit in &compilation.units {
            if config.excluded(&unit.id, "") {
                continue;
            }
            if let Some(map) = unit.version_map.as_ref().filter(|_| unit.is_root) {
                let selected = contracts
                    .iter()
                    .filter(|contract| contract.unit == unit.id)
                    .collect::<Vec<_>>();
                let pages = Changes {
                    config,
                    schemas: &site.schemas,
                    uses: &site.uses,
                    renderer: &site.renderer,
                }
                .pages(map, &selected, &notes)?;
                for page in pages {
                    site.insert(page)?;
                }
            }
        }
        site.indexes(&contracts)?;
        site.artifacts(&contracts)
    }

    fn collect_schemas(&mut self, contracts: &[Contract]) -> Result<()> {
        let mut available = BTreeMap::new();
        for contract in contracts {
            for (id, schema) in &contract.schemas {
                if available.insert(id.clone(), schema.clone()).is_some() {
                    return Err(DocumentationError::Contract(format!(
                        "duplicate schema identity '{}::{}'",
                        id.unit, id.name
                    )));
                }
            }
        }
        SchemaView::normalize(&mut available)?;
        for contract in contracts {
            for operation in contract.operations.values() {
                let link = PageLink {
                    title: format!("{} {}::{}", contract.version, contract.unit, operation.name),
                    url: self.config.url(&contract.operation_path(&operation.name)),
                };
                let mut pending = Links::references(&operation.value)
                    .iter()
                    .map(|reference| Links::resolve(&contract.origin(), reference))
                    .collect::<Result<Vec<_>>>()?;
                let mut visited = BTreeSet::new();
                while let Some(id) = pending.pop() {
                    if !visited.insert(id.clone()) {
                        continue;
                    }
                    if self.config.excluded(&id.unit, &id.logical_name()) {
                        return Err(DocumentationError::Contract(format!(
                            "published operation '{}::{}' references excluded type '{}::{}'",
                            contract.unit,
                            operation.name,
                            id.unit,
                            id.logical_name()
                        )));
                    }
                    let schema = available.get(&id).ok_or_else(|| {
                        DocumentationError::Contract(format!(
                            "missing schema reference '{}:{}:{}'",
                            id.unit, id.version, id.name
                        ))
                    })?;
                    pending.extend(
                        Links::references(schema)
                            .iter()
                            .map(|reference| Links::resolve(&id, reference))
                            .collect::<Result<Vec<_>>>()?,
                    );
                    self.schemas.insert(id.clone(), schema.clone());
                    self.uses.entry(id).or_default().push(link.clone());
                }
            }
        }
        for links in self.uses.values_mut() {
            links.sort_by(|a, b| a.url.cmp(&b.url));
            links.dedup_by(|a, b| a.url == b.url);
        }
        Ok(())
    }

    fn insert(&mut self, page: Page) -> Result<()> {
        if self.pages.insert(page.path.clone(), page.clone()).is_some() {
            return Err(DocumentationError::Contract(format!(
                "duplicate documentation output path '{}'",
                page.path
            )));
        }
        Ok(())
    }
}
