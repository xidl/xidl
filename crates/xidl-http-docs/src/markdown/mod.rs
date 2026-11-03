use crate::config::DocumentationConfig;
use crate::error::{DocumentationError, Result};
use crate::links::Links;
use crate::page::Page;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::BTreeSet;

mod symbols;

#[cfg(test)]
mod tests;

pub(crate) struct Markdown<'a> {
    pub page: &'a Page,
    pub config: &'a DocumentationConfig,
}

impl Markdown<'_> {
    pub fn safe_fragment(source: String) -> String {
        let mut escaped = String::new();
        let mut cursor = 0;
        for (event, range) in Parser::new_ext(&source, Self::options()).into_offset_iter() {
            if matches!(event, Event::Html(_) | Event::InlineHtml(_)) && range.start >= cursor {
                escaped.push_str(&source[cursor..range.start]);
                escaped.push_str(
                    &source[range.clone()]
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;"),
                );
                cursor = range.end;
            }
        }
        escaped.push_str(&source[cursor..]);
        escaped
    }

    pub fn for_site(&self) -> Result<String> {
        // Starlight renders the generated page title from frontmatter.
        let body = self
            .page
            .markdown
            .split_once('\n')
            .map_or("", |(_, body)| body);
        let events = Parser::new_ext(body, Self::options())
            .map(|event| -> Result<_> {
                Ok(match event {
                    Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }) => Event::Start(Tag::Link {
                        link_type,
                        dest_url: self.site_link(&dest_url)?.into(),
                        title,
                        id,
                    }),
                    // Only stable IDs are part of the documentation heading contract.
                    // Author-supplied classes and HTML attributes never reach the site builder.
                    Event::Start(Tag::Heading { level, id, .. }) => Event::Start(Tag::Heading {
                        level,
                        id,
                        classes: Vec::new(),
                        attrs: Vec::new(),
                    }),
                    event => event,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut output = String::new();
        pulldown_cmark_to_cmark::cmark(events.into_iter(), &mut output, Self::options())?;
        Ok(output)
    }

    pub fn destinations(markdown: &str) -> Vec<String> {
        Parser::new_ext(markdown, Self::options())
            .filter_map(|event| match event {
                Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                    Some(dest_url.to_string())
                }
                _ => None,
            })
            .collect()
    }

    pub fn anchors(markdown: &str) -> BTreeSet<String> {
        let body = markdown.split_once('\n').map_or("", |(_, body)| body);
        let mut anchors = BTreeSet::from(["_top".into()]);
        let mut slugger = github_slugger::Slugger::default();
        let mut heading = None;
        let mut text = String::new();
        for event in Parser::new_ext(body, Self::options()) {
            match event {
                Event::Start(Tag::Heading { id, .. }) => {
                    heading = Some(id.map(|id| id.to_string()));
                    text.clear();
                }
                Event::Text(value) | Event::Code(value) if heading.is_some() => {
                    text.push_str(&value)
                }
                Event::SoftBreak | Event::HardBreak if heading.is_some() => text.push('\n'),
                Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                    if let Some(id) = heading.take() {
                        anchors.insert(id.unwrap_or_else(|| slugger.slug(&text)));
                    }
                }
                _ => {}
            }
        }
        anchors
    }

    pub fn validate_examples(markdown: &str, routes: &[(String, String)]) -> Result<()> {
        let mut language = String::new();
        let mut code = String::new();
        for event in Parser::new_ext(markdown, Self::options()) {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_string(),
                        _ => String::new(),
                    };
                    code.clear();
                }
                Event::Text(text) if !language.is_empty() => code.push_str(&text),
                Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                    match language.as_str() {
                        "json" => {
                            serde_json::from_str::<serde_json::Value>(&code)?;
                        }
                        "http" | "http-request" => {
                            let line = code.lines().next().unwrap_or_default();
                            let mut words = line.split_whitespace();
                            let method = words.next().unwrap_or_default();
                            let path = words
                                .next()
                                .unwrap_or_default()
                                .split('?')
                                .next()
                                .unwrap_or_default();
                            if !routes.iter().any(|(verb, route)| {
                                verb == method && Self::route_matches(route, path)
                            }) {
                                return Err(DocumentationError::Contract(format!(
                                    "request example '{line}' does not match a published HTTP route"
                                )));
                            }
                        }
                        _ => {}
                    }
                    language.clear();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn route_matches(route: &str, path: &str) -> bool {
        let template: Vec<_> = route.split('/').collect();
        let actual: Vec<_> = path.split('/').collect();
        template.len() == actual.len()
            && template.iter().zip(actual).all(|(part, value)| {
                *part == value
                    || (part.starts_with('{') && part.ends_with('}') && !value.is_empty())
            })
    }

    fn options() -> Options {
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_HEADING_ATTRIBUTES
    }

    pub fn site_link(&self, url: &str) -> Result<String> {
        let Some((path, _)) = self.config.destination(&self.page.path, url)? else {
            return Ok(url.into());
        };
        let path = if path.ends_with(".md") {
            Links::site_path(&path)
        } else {
            path
        };
        let suffix = url.find(['?', '#']).map_or("", |position| &url[position..]);
        Ok(format!("{}{suffix}", self.config.browser_url(&path)?))
    }
}
