mod lines;

use crate::config::DocumentationConfig;
use crate::contract::Contract;
use crate::error::{DocumentationError, Result};
use crate::links::{Links, SchemaId};
use crate::notes::ReleaseNote;
use crate::page::{ChangeFact, ChangePage, Page, PageContent, PageLink};
use crate::render::Renderer;
use crate::schema::SchemaView;
use crate::summary::ChangeSummary;
use crate::wire::WireSurface;
use lines::ComparisonLines;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::versioning::map::VersionMap;

pub(crate) struct Changes<'a> {
    pub config: &'a DocumentationConfig,
    pub schemas: &'a BTreeMap<SchemaId, Value>,
    pub uses: &'a BTreeMap<SchemaId, Vec<PageLink>>,
    pub renderer: &'a Renderer,
}

impl Changes<'_> {
    pub fn pages(
        &self,
        map: &VersionMap,
        contracts: &[&Contract],
        notes: &[ReleaseNote],
    ) -> Result<Vec<Page>> {
        let mut pages = Vec::new();
        for step in &map.steps {
            let old = contracts
                .iter()
                .find(|contract| contract.version == step.from);
            let new = contracts
                .iter()
                .find(|contract| contract.version == step.to);
            let (Some(old), Some(new)) = (old, new) else {
                continue;
            };
            if old.operations.is_empty() && new.operations.is_empty() {
                continue;
            }
            let mut facts = self.type_changes(old, new)?;
            facts.extend(self.operation_changes(old, new)?);
            facts.sort_by(|a, b| a.subject.cmp(&b.subject));
            let note = notes
                .iter()
                .find(|note| note.unit == old.unit && note.from == step.from && note.to == step.to);
            if self.config.strict && !facts.is_empty() && !note.is_some_and(ReleaseNote::ready) {
                return Err(DocumentationError::Contract(format!(
                    "release validation '{}': {} to {} has unreviewed changes or missing Impact/Upgrade",
                    old.unit, step.from, step.to
                )));
            }
            let playgrounds = [old, new]
                .into_iter()
                .filter(|contract| !contract.operations.is_empty())
                .map(|contract| PageLink {
                    title: format!("Explore {} HTTP API", contract.wire_version),
                    url: self
                        .config
                        .url(&Links::playground(&contract.unit, &contract.version)),
                })
                .collect();
            let view = ChangePage {
                title: format!("{}: {} → {}", old.unit, step.from, step.to),
                unit: old.unit.clone(),
                from_version: old.version.clone(),
                to_version: new.version.clone(),
                from: old.wire_version.clone(),
                to: new.wire_version.clone(),
                facts,
                playgrounds,
                author_title: note
                    .map_or("No author explanation provided", |note| note.title.as_str())
                    .into(),
                author_body: note.map_or("", |note| note.markdown.as_str()).into(),
                impact: note
                    .filter(|note| !note.impact.trim().is_empty())
                    .map_or("Needs review / No business impact provided.", |note| {
                        note.impact.as_str()
                    })
                    .into(),
                upgrade: note
                    .filter(|note| !note.upgrade.trim().is_empty())
                    .map_or("No migration steps provided.", |note| note.upgrade.as_str())
                    .into(),
                reviewed: note.is_some_and(ReleaseNote::ready),
            };
            pages.push(Page {
                path: format!(
                    "changelog/{}/{}-to-{}.md",
                    Links::segment(&old.unit),
                    Links::segment(&step.from),
                    Links::segment(&step.to)
                ),
                title: view.title.clone(),
                group: "Changelog".into(),
                markdown: self.renderer.template("changes.md", &view)?,
                content: PageContent::Changes(Box::new(view)),
            });
        }
        Ok(pages)
    }

    fn type_changes(&self, old: &Contract, new: &Contract) -> Result<Vec<ChangeFact>> {
        let mut facts = Vec::new();
        let names: BTreeSet<_> = self
            .schemas
            .keys()
            .filter(|id| {
                id.unit == old.unit && (id.version == old.version || id.version == new.version)
            })
            .map(SchemaId::logical_name)
            .collect();
        for name in names {
            let previous = self.find_schema(old, &name);
            let next = self.find_schema(new, &name);
            let old_link = previous.map(|(id, _)| self.schema_link(id));
            let new_link = next.map(|(id, _)| self.schema_link(id));
            let uses = previous
                .into_iter()
                .chain(next)
                .flat_map(|(id, _)| self.uses.get(id).into_iter().flatten().cloned())
                .collect::<Vec<_>>();
            let evidence = ChangeLinks {
                declaration: name.clone(),
                before: old.idl.declaration(&name),
                after: new.idl.declaration(&name),
                old_version: old.version.clone(),
                new_version: new.version.clone(),
                old: old_link.clone(),
                new: new_link.clone(),
                uses,
            };
            match (previous, next) {
                (Some((old_id, before)), Some((new_id, after))) => {
                    let old_fields = self.field_values(old_id, before)?;
                    let new_fields = self.field_values(new_id, after)?;
                    for field in old_fields
                        .keys()
                        .chain(new_fields.keys())
                        .collect::<BTreeSet<_>>()
                    {
                        let mut links = evidence.clone();
                        if !old_fields.contains_key(field) {
                            links.old = None;
                        }
                        if !new_fields.contains_key(field) {
                            links.new = None;
                        }
                        for link in links.old.iter_mut().chain(links.new.iter_mut()) {
                            link.url
                                .push_str(&format!("#field-{}", Links::segment(field)));
                        }
                        if let Some(fact) = self.fact(
                            format!("{name}.{field}"),
                            (old_fields.get(field), new_fields.get(field)),
                            links,
                        )? {
                            facts.push(fact);
                        }
                    }
                    // Members have their own facts; retain enum/union/base/constraint changes once.
                    let mut before = before.clone();
                    let mut after = after.clone();
                    for value in [&mut before, &mut after] {
                        if let Some(object) = value.as_object_mut() {
                            object.remove("properties");
                            object.remove("required");
                        }
                    }
                    if let Some(fact) =
                        self.fact(name.clone(), (Some(&before), Some(&after)), evidence)?
                    {
                        facts.push(fact);
                    }
                }
                _ => {
                    if let Some(fact) = self.fact(
                        name,
                        (
                            previous.map(|(_, value)| value),
                            next.map(|(_, value)| value),
                        ),
                        evidence,
                    )? {
                        facts.push(fact);
                    }
                }
            }
        }
        Ok(facts)
    }

    fn operation_changes(&self, old: &Contract, new: &Contract) -> Result<Vec<ChangeFact>> {
        let mut facts = Vec::new();
        for name in old
            .operations
            .keys()
            .chain(new.operations.keys())
            .collect::<BTreeSet<_>>()
        {
            let previous = old.operations.get(name);
            let next = new.operations.get(name);
            let before = previous.map(|op| serde_json::json!({ "routes": op.routes, "publication": op.publication, "contract": WireSurface::operation(&op.value, old.version_parameter.as_ref()), "securitySchemes": op.security_schemes }));
            let after = next.map(|op| serde_json::json!({ "routes": op.routes, "publication": op.publication, "contract": WireSurface::operation(&op.value, new.version_parameter.as_ref()), "securitySchemes": op.security_schemes }));
            let links = ChangeLinks {
                declaration: name.to_string(),
                before: old.idl.declaration(name),
                after: new.idl.declaration(name),
                old_version: old.version.clone(),
                new_version: new.version.clone(),
                old: previous.map(|_| PageLink {
                    title: format!("{} {name}", old.version),
                    url: self.config.url(&old.operation_path(name)),
                }),
                new: next.map(|_| PageLink {
                    title: format!("{} {name}", new.version),
                    url: self.config.url(&new.operation_path(name)),
                }),
                uses: Vec::new(),
            };
            if let Some(fact) = self.fact(name.clone(), (before.as_ref(), after.as_ref()), links)? {
                facts.push(fact);
            }
        }
        Ok(facts)
    }

    fn fact(
        &self,
        subject: String,
        values: (Option<&Value>, Option<&Value>),
        links: ChangeLinks,
    ) -> Result<Option<ChangeFact>> {
        let (before, after) = values;
        let previous = before.map(|value| {
            let mut value = WireSurface::normalize(value);
            WireSurface::local_references(&mut value, &links.old_version);
            value
        });
        let next = after.map(|value| {
            let mut value = WireSurface::normalize(value);
            WireSurface::local_references(&mut value, &links.new_version);
            value
        });
        if previous == next {
            return Ok(None);
        }
        let (before_summary, after_summary) = ChangeSummary::pair(previous.as_ref(), next.as_ref());
        let change = match (before, after) {
            (None, _) => "Added",
            (_, None) => "Removed",
            _ => "Changed",
        };
        let before = links.before;
        let after = links.after;
        let no_idl_changes = before.is_some() && before == after;
        let lines = ComparisonLines::between(
            before.as_deref().unwrap_or(""),
            after.as_deref().unwrap_or(""),
        );
        Ok(Some(ChangeFact {
            declaration: links.declaration,
            no_idl_changes,
            before_lines: lines.before,
            after_lines: lines.after,
            anchor: format!("change-{}", Links::segment(&subject)),
            subject,
            change: change.into(),
            before_summary,
            after_summary,
            before,
            after,
            assessment:
                "Needs review: transport direction, requiredness and caller usage require review."
                    .into(),
            old: links.old,
            new: links.new,
            uses: links.uses,
        }))
    }

    fn find_schema<'a>(
        &'a self,
        contract: &Contract,
        name: &str,
    ) -> Option<(&'a SchemaId, &'a Value)> {
        self.schemas.iter().find(|(id, _)| {
            id.unit == contract.unit && id.version == contract.version && id.logical_name() == name
        })
    }

    fn field_values(&self, id: &SchemaId, value: &Value) -> Result<BTreeMap<String, Value>> {
        SchemaView { origin: id, schemas: self.schemas, config: self.config }.fields(value)?
            .into_iter().map(|row| Ok((row.name, serde_json::json!({"required":row.required,"schema":serde_json::from_str::<Value>(&row.details)?})))).collect()
    }

    fn schema_link(&self, id: &SchemaId) -> PageLink {
        PageLink {
            title: format!("{} {}", id.version, id.logical_name()),
            url: self.config.url(&id.path()),
        }
    }
}

#[derive(Clone)]
struct ChangeLinks {
    declaration: String,
    before: Option<String>,
    after: Option<String>,
    old_version: String,
    new_version: String,
    old: Option<PageLink>,
    new: Option<PageLink>,
    uses: Vec<PageLink>,
}
