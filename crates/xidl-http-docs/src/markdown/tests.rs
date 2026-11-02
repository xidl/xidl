use super::Markdown;
use crate::config::DocumentationConfig;
use crate::page::{IndexPage, Page, PageContent};

fn page(markdown: &str) -> Page {
    Page {
        path: "api/catalog/v1/index.md".into(),
        title: "Catalog".into(),
        group: "Versions".into(),
        markdown: markdown.into(),
        content: PageContent::Index(Box::new(IndexPage {
            title: "Catalog".into(),
            overview: false,
            scope: None,
            links: vec![],
            releases: vec![],
        })),
    }
}

#[test]
fn browser_projection_rewrites_reference_links_preserves_code_and_drops_heading_attributes() {
    let page = page(
        "# Catalog\n\n## Fields {#field-id .custom onclick=alert(1)}\n\n[Product][product] and [external](https://other.test/guide.md).\n\n[product]: ../v2/types/_Product.md?source=change#field-name\n\n```http\n# preserve this comment\nGET /products/42\n```\n",
    );
    let config = DocumentationConfig {
        base_url: "https://docs.example.test/reference/".into(),
        ..DocumentationConfig::default()
    };
    let output = Markdown {
        page: &page,
        config: &config,
    }
    .for_site()
    .unwrap();
    assert!(
        output.contains("/reference/api/catalog/v2/types/_Product/?source=change#field-name"),
        "{output}"
    );
    assert!(!output.contains("https://docs.example.test"), "{output}");
    assert!(output.contains("https://other.test/guide.md"));
    let markdown = Markdown {
        page: &page,
        config: &config,
    };
    assert_eq!(
        markdown
            .site_link("https://docs.example.test/reference/openapi/catalog/v1.json?download=1")
            .unwrap(),
        "/reference/openapi/catalog/v1.json?download=1"
    );
    assert!(output.contains("# preserve this comment\nGET /products/42"));
    assert!(pulldown_cmark::Parser::new_ext(&output, Markdown::options()).any(|event| matches!(event,
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { id: Some(id), classes, attrs, .. })
            if id.as_ref() == "field-id" && classes.is_empty() && attrs.is_empty()
    )));
    assert!(!output.contains("onclick"), "{output}");
    assert!(!output.contains(".custom"));
    assert!(!output.starts_with("# Catalog"));
}

#[test]
fn anchors_follow_real_headings_and_ignore_examples_with_anchor_looking_text() {
    let anchors = Markdown::anchors(
        "# Generated title\n\n## Upgrade\n\n## Upgrade\n\n### 变化 `name`\n\n### Explicit { #custom }\n\n```markdown\n## Fake {#not-a-heading}\n```\n",
    );
    assert_eq!(
        anchors,
        ["_top", "upgrade", "upgrade-1", "变化-name", "custom"]
            .map(str::to_owned)
            .into_iter()
            .collect()
    );
}

#[test]
fn author_html_is_literal_but_fenced_examples_preserve_original_source() {
    let source = "<script>window.xidlUnsafe = true</script>\n\n<img src=x onerror=alert(1)>\n\n```html\n<script>example()</script>\n```\n";
    let safe = Markdown::safe_fragment(source.into());
    assert!(safe.contains("&lt;script&gt;window.xidlUnsafe = true&lt;/script&gt;"));
    assert!(safe.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(safe.contains("```html\n<script>example()</script>\n```"));
}

#[test]
fn example_metadata_keeps_json_and_parameterized_http_routes_validated() {
    let routes = vec![("GET".into(), "/invoices/{id}".into())];
    Markdown::validate_examples(
        "```http-request title=\"Request\"\nGET /invoices/in_1?expand=payments HTTP/1.1\n```\n\n```json title=\"Response\"\n{\"id\":\"in_1\"}\n```\n",
        &routes,
    )
    .unwrap();
    for request in [
        "DELETE /invoices/in_1",
        "GET /invoices/",
        "GET /invoices/in_1/extra",
    ] {
        let markdown = format!("```http title=\"Request\"\n{request}\n```\n");
        assert!(matches!(
            Markdown::validate_examples(&markdown, &routes),
            Err(crate::error::DocumentationError::Contract(_))
        ));
    }
    assert!(matches!(
        Markdown::validate_examples("```json title=\"Response\"\n{invalid}\n```\n", &routes),
        Err(crate::error::DocumentationError::Json(_))
    ));
}

#[test]
fn deployment_links_preserve_external_paths_and_reject_executable_schemes() {
    let page = page("# Catalog\n");
    let config = DocumentationConfig {
        base_url: "https://docs.example.test/reference/".into(),
        ..DocumentationConfig::default()
    };
    let markdown = Markdown {
        page: &page,
        config: &config,
    };
    assert_eq!(
        markdown
            .site_link("https://docs.example.test/outside.md")
            .unwrap(),
        "https://docs.example.test/outside.md"
    );
    assert_eq!(
        markdown.site_link("../../../index.md").unwrap(),
        "/reference/"
    );
    assert!(
        markdown
            .site_link("javascript:alert(1)")
            .unwrap_err()
            .to_string()
            .contains("unsupported documentation link")
    );
}
