use super::theme::Base16Theme;
use miette::highlighters::{
    Highlighter as MietteHighlighter, HighlighterState as MietteHighlighterState,
};
use owo_colors::Style;
use tree_sitter::Language;
use tree_sitter_highlight::{
    Highlight, HighlightConfiguration, HighlightEvent, Highlighter as TreeSitterHighlighter,
};

#[derive(Debug)]
pub struct TreeSitterMietteHighlighter {
    language: Language,
    language_name: &'static str,
    highlight_query: &'static str,
    injection_query: &'static str,
    locals_query: &'static str,
}

impl TreeSitterMietteHighlighter {
    pub fn new_idl() -> Self {
        Self {
            language: tree_sitter_idl::language(),
            language_name: "idl",
            highlight_query: tree_sitter_idl::HIGHLIGHTS_QUERY,
            injection_query: tree_sitter_idl::INJECTIONS_QUERY,
            locals_query: "",
        }
    }
}

struct TreeSitterMietteHighlighterState {
    highlighter: TreeSitterHighlighter,
    config: HighlightConfiguration,
}

impl MietteHighlighter for TreeSitterMietteHighlighter {
    fn start_highlighter_state<'h>(
        &'h self,
        source: &dyn miette::SpanContents<'_>,
    ) -> Box<dyn MietteHighlighterState + 'h> {
        let language_is_idl = source
            .language()
            .map(|lang| lang.eq_ignore_ascii_case("idl"))
            .unwrap_or(false);

        tracing::debug!("language is: {:?}", source.language());
        if !language_is_idl {
            return Box::new(miette::highlighters::BlankHighlighterState);
        }

        let Ok(config) = build_highlight_configuration(
            self.language.clone(),
            self.language_name,
            self.highlight_query,
            self.injection_query,
            self.locals_query,
        ) else {
            return Box::new(miette::highlighters::BlankHighlighterState);
        };

        Box::new(TreeSitterMietteHighlighterState {
            highlighter: TreeSitterHighlighter::new(),
            config,
        })
    }
}

impl MietteHighlighterState for TreeSitterMietteHighlighterState {
    fn highlight_line<'s>(&mut self, line: &'s str) -> Vec<owo_colors::Styled<&'s str>> {
        let Ok(events) = self
            .highlighter
            .highlight(&self.config, line.as_bytes(), None, |_| None)
        else {
            return vec![Style::default().style(line)];
        };

        let names = self.config.names();
        let mut styled: Vec<owo_colors::Styled<&'s str>> = Vec::new();
        let mut highlight_stack: Vec<Highlight> = Vec::new();
        for event in events {
            let Ok(event) = event else {
                return vec![Style::default().style(line)];
            };

            match event {
                HighlightEvent::HighlightStart(highlight) => highlight_stack.push(highlight),
                HighlightEvent::HighlightEnd => {
                    let _ = highlight_stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    if start >= end || end > line.len() {
                        continue;
                    }
                    let segment = &line[start..end];
                    let style = highlight_stack
                        .last()
                        .and_then(|highlight| names.get(highlight.0))
                        .map(|capture_name| {
                            let rgb = super::colors::color_for_capture(
                                &Base16Theme::dracula(),
                                capture_name,
                            );
                            Style::new().truecolor(rgb.r, rgb.g, rgb.b)
                        })
                        .unwrap_or_default();
                    styled.push(style.style(segment));
                }
            }
        }

        if styled.is_empty() {
            return vec![Style::default().style(line)];
        }

        styled
    }
}

fn build_highlight_configuration(
    language: Language,
    language_name: &str,
    highlight_query: &str,
    injection_query: &str,
    locals_query: &str,
) -> Result<HighlightConfiguration, tree_sitter::QueryError> {
    let mut config = HighlightConfiguration::new(
        language,
        language_name,
        highlight_query,
        injection_query,
        locals_query,
    )?;
    let names = config
        .names()
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    config.configure(&names);
    Ok(config)
}
