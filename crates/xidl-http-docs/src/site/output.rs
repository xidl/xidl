use super::Site;
use crate::contract::Contract;
use crate::error::{DocumentationError, Result};
use crate::links::Links;
use crate::markdown::Markdown;
use crate::page::PageLink;
use std::collections::BTreeMap;
use xidlc::jsonrpc::{Artifact, ArtifactFile};

impl Site<'_> {
    pub(super) fn artifacts(self, contracts: &[Contract]) -> Result<Vec<Artifact>> {
        self.validate_links(contracts)?;
        let mut artifacts = self.starlight(contracts)?;
        artifacts.extend(self.playgrounds(contracts)?);
        let mut full = String::new();
        for page in self.pages.values() {
            full.push_str(&page.markdown);
            full.push_str("\n\n");
            if full.len() > self.config.full_max_bytes {
                return Err(DocumentationError::Contract(format!(
                    "llms-full.txt exceeds full_max_bytes {}; no truncated output was written",
                    self.config.full_max_bytes
                )));
            }
            artifacts.insert(format!("public/{}", page.path), page.markdown.clone());
        }
        let groups: BTreeMap<String, Vec<PageLink>> = ["Versions", "Changelog"]
            .iter()
            .map(|group| {
                (
                    group.to_string(),
                    self.pages
                        .values()
                        .filter(|page| page.group == *group)
                        .map(|page| PageLink {
                            title: page.title.clone(),
                            url: self.config.url(&page.path),
                        })
                        .collect(),
                )
            })
            .collect();
        let title = self
            .pages
            .get("index.md")
            .map(|page| page.title.as_str())
            .unwrap_or("API reference");
        let index = self.renderer.template(
            "llms.txt",
            minijinja::context! { title, groups, home => self.config.url("index.md") },
        )?;
        if index.len() > self.config.index_max_bytes {
            return Err(DocumentationError::Contract("llms.txt exceeds index_max_bytes; split the publication or raise the explicit limit".into()));
        }
        artifacts.insert("public/llms.txt".into(), index);
        artifacts.insert("public/llms-full.txt".into(), full);
        Ok(artifacts
            .into_iter()
            .map(|(path, content)| Artifact::File(ArtifactFile { path, content }))
            .collect())
    }

    fn validate_links(&self, contracts: &[Contract]) -> Result<()> {
        let artifacts = contracts
            .iter()
            .filter(|contract| !contract.operations.is_empty())
            .flat_map(|contract| {
                [
                    Links::playground(&contract.unit, &contract.version),
                    Links::specification(&contract.unit, &contract.version),
                ]
            })
            .collect::<std::collections::BTreeSet<_>>();
        let anchors: BTreeMap<_, _> = self
            .pages
            .iter()
            .map(|(path, page)| (path, Markdown::anchors(&page.markdown)))
            .collect();
        for page in self.pages.values() {
            for url in Markdown::destinations(&page.markdown) {
                if let Some((path, anchor)) = self.config.destination(&page.path, &url)? {
                    if artifacts.contains(&path) && anchor.is_none() {
                        continue;
                    }
                    let path = path.replace(".html", ".md");
                    let target = anchors.get(&path).ok_or_else(|| {
                        DocumentationError::Contract(format!(
                            "page '{}' links to missing page '{url}'",
                            page.path
                        ))
                    })?;
                    if let Some(anchor) = anchor {
                        if !target.contains(&anchor) {
                            return Err(DocumentationError::Contract(format!(
                                "page '{}' links to missing anchor '{url}'",
                                page.path
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
