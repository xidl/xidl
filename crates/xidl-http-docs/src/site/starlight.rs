use super::Site;
use crate::contract::Contract;
use crate::error::Result;
use crate::links::Links;
use crate::markdown::Markdown;
use crate::page::{PageContent, PageLink};
use serde_json::{Value, json};
use std::collections::BTreeMap;

impl Site<'_> {
    pub(super) fn starlight(&self, contracts: &[Contract]) -> Result<BTreeMap<String, String>> {
        let mut files = BTreeMap::new();
        for (path, source) in [
            ("package.json", include_str!("../../frontend/package.json")),
            (
                "pnpm-lock.yaml",
                include_str!("../../frontend/pnpm-lock.yaml"),
            ),
            (
                "pnpm-workspace.yaml",
                include_str!("../../frontend/pnpm-workspace.yaml"),
            ),
            (
                "astro.config.mjs",
                include_str!("../../frontend/astro.config.mjs"),
            ),
            (
                "tsconfig.json",
                include_str!("../../frontend/tsconfig.json"),
            ),
            (
                "src/content.config.ts",
                include_str!("../../frontend/src/content.config.ts"),
            ),
            (
                "src/styles/global.css",
                include_str!("../../frontend/src/styles/global.css"),
            ),
            (
                "src/content/i18n/en.json",
                include_str!("../../frontend/src/content/i18n/en.json"),
            ),
            (
                "public/favicon.svg",
                include_str!("../../frontend/public/favicon.svg"),
            ),
            (".gitignore", include_str!("../../frontend/.gitignore")),
            ("README.md", include_str!("../../frontend/README.md")),
            (
                "scripts/prepare-scalar.mjs",
                include_str!("../../frontend/scripts/prepare-scalar.mjs"),
            ),
            (
                "Scalar.LICENSE.txt",
                include_str!("../../frontend/Scalar.LICENSE.txt"),
            ),
            (
                "src/shiki/idl.mjs",
                include_str!("../../frontend/src/shiki/idl.mjs"),
            ),
            (
                "src/components/Head.astro",
                include_str!("../../frontend/src/components/Head.astro"),
            ),
            (
                "src/scalar-theme.ts",
                include_str!("../../frontend/src/scalar-theme.ts"),
            ),
        ] {
            files.insert(path.into(), source.into());
        }
        files.insert(
            "src/route-data.ts".into(),
            include_str!("../../frontend/src/route-data.ts").into(),
        );
        files.insert("xidl-site.json".into(), self.publication(contracts)?);
        for page in self.pages.values() {
            let body = Markdown {
                page,
                config: self.config,
            }
            .for_site()?;
            let versions = self
                .pages
                .values()
                .filter(|other| page.same_reference(other))
                .map(|other| {
                    Ok(PageLink {
                        title: format!("{} · {}", other.group, other.title),
                        url: self.config.browser_url(&Links::site_path(&other.path))?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let mut related = Vec::new();
            let urls = self
                .pages
                .values()
                .filter(|other| page.same_reference(other))
                .map(|other| self.config.url(&other.path))
                .collect::<Vec<_>>();
            for change in self.pages.values() {
                if let PageContent::Changes(view) = &change.content {
                    for fact in &view.facts {
                        if fact.old.iter().chain(&fact.new).any(|link| {
                            urls.iter()
                                .any(|url| link.url.split('#').next() == Some(url.as_str()))
                        }) {
                            related.push(PageLink {
                                title: format!("{} → {} · {}", view.from, view.to, fact.subject),
                                url: format!(
                                    "{}#{}",
                                    self.config.browser_url(&Links::site_path(&change.path))?,
                                    fact.anchor
                                ),
                            });
                        }
                    }
                }
            }
            let deployment = self.config.base()?;
            let (playground, specification): (Option<String>, Option<String>) = page
                .reference_scope()
                .filter(|(unit, version)| {
                    contracts.iter().any(|contract| {
                        contract.unit == *unit
                            && contract.version == *version
                            && !contract.operations.is_empty()
                    })
                })
                .map(|(unit, version)| {
                    (
                        format!("{}{}", deployment.path(), Links::playground(unit, version)),
                        format!(
                            "{}{}",
                            deployment.path(),
                            Links::specification(unit, version)
                        ),
                    )
                })
                .unzip();
            let content = self.renderer.template(
                "starlight.md",
                minijinja::context! {
                    title => page.title, slug => Links::slug(&page.path), body,
                    markdown_url => self.config.browser_url(&page.path)?,
                    llms_url => self.config.browser_url("llms.txt")?, versions, related,
                    playground, specification,
                },
            )?;
            // The default Starlight loader ignores leading-underscore filenames.
            // The explicit slug preserves every canonical IDL path regardless of filename.
            let (directory, name) = page.path.rsplit_once('/').unwrap_or(("", &page.path));
            let path =
                format!("src/content/docs/{directory}/page-{name}").replace("docs//", "docs/");
            files.insert(path, content);
        }
        Ok(files)
    }

    fn publication(&self, contracts: &[Contract]) -> Result<String> {
        let mut sidebar = vec![json!({"label":"Overview", "slug":"index"})];
        let changes = self
            .pages
            .values()
            .filter(|page| page.group == "Changelog")
            .map(|page| json!({"label":page.title,"slug":Links::slug(&page.path)}))
            .collect::<Vec<_>>();
        if !changes.is_empty() {
            sidebar.push(json!({"label":"Version changes", "items":changes}));
        }
        for contract in contracts {
            let items = self
                .pages
                .values()
                .filter(|page| {
                    page.reference_scope()
                        == Some((contract.unit.as_str(), contract.version.as_str()))
                })
                .map(|page| json!({"label":page.title,"slug":Links::slug(&page.path)}))
                .collect::<Vec<_>>();
            if !items.is_empty() {
                sidebar.push(json!({"label":format!("{} / {}", contract.unit, contract.version),"collapsed":true,"items":items}));
            }
        }
        let base = self.config.base()?;
        let site = if self.config.base_url.starts_with("http") {
            Value::String(base.origin().ascii_serialization())
        } else {
            Value::Null
        };
        Ok(serde_json::to_string_pretty(&json!({
            "title": self.pages.get("index.md").map(|page| page.title.as_str()).unwrap_or("API reference"),
            "site":site, "base":base.path(), "sidebar":sidebar,
            "playgrounds":contracts.iter().filter(|contract| !contract.operations.is_empty()).map(|contract| json!({
                "unit":contract.unit,"version":contract.version,
                "title":format!("{} · {}", contract.title, contract.wire_version),
                "pathname":format!("/{}",Links::playground(&contract.unit,&contract.version)),
                "specPath":Links::specification(&contract.unit,&contract.version),
            })).collect::<Vec<_>>(),
        }))?)
    }
}
