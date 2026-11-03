use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PageLink {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Page {
    pub path: String,
    pub title: String,
    pub group: String,
    pub markdown: String,
    pub content: PageContent,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) enum PageContent {
    Index(Box<IndexPage>),
    Operation(Box<OperationPage>),
    Type(Box<TypePage>),
    Changes(Box<ChangePage>),
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChangeFact {
    pub anchor: String,
    pub subject: String,
    pub change: String,
    pub declaration: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub no_idl_changes: bool,
    pub before_lines: String,
    pub after_lines: String,
    pub before_summary: String,
    pub after_summary: String,
    pub assessment: String,
    pub old: Option<PageLink>,
    pub new: Option<PageLink>,
    pub uses: Vec<PageLink>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChangePage {
    pub title: String,
    pub unit: String,
    pub from_version: String,
    pub to_version: String,
    pub from: String,
    pub to: String,
    pub facts: Vec<ChangeFact>,
    pub playgrounds: Vec<PageLink>,
    pub author_title: String,
    pub author_body: String,
    pub impact: String,
    pub upgrade: String,
    pub reviewed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IndexPage {
    pub title: String,
    pub overview: bool,
    pub scope: Option<ReferenceScope>,
    pub links: Vec<PageLink>,
    pub releases: Vec<ReleaseSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReferenceScope {
    pub unit: String,
    pub version: String,
}

impl Page {
    pub fn reference_scope(&self) -> Option<(&str, &str)> {
        match &self.content {
            PageContent::Operation(view) => Some((&view.unit, &view.version)),
            PageContent::Type(view) => Some((&view.unit, &view.version)),
            PageContent::Index(view) => view
                .scope
                .as_ref()
                .map(|scope| (scope.unit.as_str(), scope.version.as_str())),
            PageContent::Changes(_) => None,
        }
    }

    pub fn same_reference(&self, other: &Page) -> bool {
        match (&self.content, &other.content) {
            (PageContent::Operation(a), PageContent::Operation(b)) => {
                a.unit == b.unit && a.title == b.title
            }
            (PageContent::Type(a), PageContent::Type(b)) => a.unit == b.unit && a.title == b.title,
            (PageContent::Index(a), PageContent::Index(b)) => a
                .scope
                .as_ref()
                .zip(b.scope.as_ref())
                .is_some_and(|(a, b)| a.unit == b.unit),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReleaseSummary {
    pub title: String,
    pub from: String,
    pub to: String,
    pub url: String,
    pub reviewed: bool,
    pub changes: Vec<PageLink>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReferenceRow {
    pub name: String,
    pub anchor: String,
    pub location: String,
    pub required: bool,
    pub ty: String,
    pub description: String,
    #[serde(skip)]
    pub documentation_origin: crate::links::SchemaId,
    pub details: String,
    pub links: Vec<PageLink>,
    pub deprecated: bool,
    pub lifecycle: String,
    pub declared_default: Option<String>,
    pub constraints: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OperationPage {
    pub title: String,
    pub unit: String,
    pub version: String,
    pub wire_version: String,
    pub description: String,
    pub interface_description: String,
    pub method: String,
    pub route: String,
    pub publication: String,
    pub deprecated: bool,
    pub lifecycle: String,
    pub parameters: Vec<ReferenceRow>,
    pub request: String,
    pub responses: String,
    pub security: String,
    pub definitions: Vec<PageLink>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TypePage {
    pub title: String,
    pub unit: String,
    pub version: String,
    pub description: String,
    pub deprecated: bool,
    pub lifecycle: String,
    pub fields: Vec<ReferenceRow>,
    pub schema: String,
    pub definitions: Vec<PageLink>,
    pub uses: Vec<PageLink>,
}
