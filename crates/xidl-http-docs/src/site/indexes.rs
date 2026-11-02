use super::Site;
use crate::contract::Contract;
use crate::error::Result;
use crate::links::Links;
use crate::page::{IndexPage, Page, PageContent, PageLink, ReferenceScope, ReleaseSummary};

impl Site<'_> {
    pub(super) fn indexes(&mut self, contracts: &[Contract]) -> Result<()> {
        let mut versions = Vec::new();
        for contract in contracts {
            let group = format!("{} / {}", contract.unit, contract.version);
            let links = self
                .pages
                .values()
                .filter(|page| page.group == group)
                .map(|page| PageLink {
                    title: page.title.clone(),
                    url: self.config.url(&page.path),
                })
                .collect::<Vec<_>>();
            if links.is_empty() {
                continue;
            }
            let title = format!(
                "{} — {} ({})",
                contract.title, contract.version, contract.wire_version
            );
            let path = format!(
                "api/{}/{}/index.md",
                Links::segment(&contract.unit),
                Links::segment(&contract.version)
            );
            let view = IndexPage {
                title: title.clone(),
                overview: false,
                scope: Some(ReferenceScope {
                    unit: contract.unit.clone(),
                    version: contract.version.clone(),
                }),
                links,
                releases: Vec::new(),
            };
            let markdown = self.renderer.template("index.md", &view)?;
            versions.push(PageLink {
                title: title.clone(),
                url: self.config.url(&path),
            });
            self.insert(Page {
                path,
                title,
                group: "Versions".into(),
                markdown,
                content: PageContent::Index(Box::new(view)),
            })?;
        }
        let mut releases = self
            .pages
            .values()
            .filter_map(|page| {
                let PageContent::Changes(view) = &page.content else {
                    return None;
                };
                let order = contracts.iter().position(|contract| {
                    contract.unit == view.unit && contract.version == view.to_version
                });
                Some((
                    order,
                    ReleaseSummary {
                        title: page.title.clone(),
                        from: view.from.clone(),
                        to: view.to.clone(),
                        url: self.config.url(&page.path),
                        reviewed: view.reviewed,
                        changes: view
                            .facts
                            .iter()
                            .map(|fact| PageLink {
                                title: format!("{} — {}", fact.subject, fact.change),
                                url: format!("{}#{}", self.config.url(&page.path), fact.anchor),
                            })
                            .collect(),
                    },
                ))
            })
            .collect::<Vec<_>>();
        releases.sort_by(|a, b| b.0.cmp(&a.0));
        let releases = releases.into_iter().map(|(_, release)| release).collect();
        let title = self
            .config
            .title
            .clone()
            .or_else(|| contracts.first().map(|contract| contract.title.clone()))
            .unwrap_or_else(|| "API reference".into());
        let view = IndexPage {
            title: title.clone(),
            overview: true,
            scope: None,
            links: versions,
            releases,
        };
        let markdown = self.renderer.template("index.md", &view)?;
        self.insert(Page {
            path: "index.md".into(),
            title,
            group: "Overview".into(),
            markdown,
            content: PageContent::Index(Box::new(view)),
        })
    }
}
