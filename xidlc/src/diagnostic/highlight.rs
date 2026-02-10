use super::theme::Base16Theme;
use miette::highlighters::{
    Highlighter as MietteHighlighter, HighlighterState as MietteHighlighterState,
};
use owo_colors::Style;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug)]
pub struct TreeSitterMietteHighlighter {
    language: tree_sitter::Language,
    highlight_query: &'static str,
}

impl TreeSitterMietteHighlighter {
    pub fn new_idl() -> Self {
        Self {
            language: tree_sitter_idl::language(),
            highlight_query: tree_sitter_idl::HIGHLIGHTS_QUERY,
        }
    }
}
struct TreeSitterMietteHighlighterState {
    language: tree_sitter::Language,
    highlight_query: &'static str,
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

        println!("language is idl: {:?}", source.language());
        if !language_is_idl {
            return Box::new(miette::highlighters::BlankHighlighterState);
        }

        Box::new(TreeSitterMietteHighlighterState {
            language: self.language.clone(),
            highlight_query: self.highlight_query,
        })
    }
}

impl MietteHighlighterState for TreeSitterMietteHighlighterState {
    fn highlight_line<'s>(&mut self, line: &'s str) -> Vec<owo_colors::Styled<&'s str>> {
        let mut parser = Parser::new();
        if parser.set_language(&self.language).is_err() {
            return vec![Style::default().style(line)];
        }

        let Some(tree) = parser.parse(line, None) else {
            return vec![Style::default().style(line)];
        };

        let language = self.language.clone();
        let Ok(query) = Query::new(&language, self.highlight_query) else {
            return vec![Style::default().style(line)];
        };

        let root = tree.root_node();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, line.as_bytes());
        let mut spans: Vec<(usize, usize, usize)> = Vec::new();

        while let Some(matched) = matches.next() {
            for capture in matched.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();
                if end <= start || start > line.len() || end > line.len() {
                    continue;
                }
                spans.push((start, end, capture.index as usize));
            }
        }

        if spans.is_empty() {
            return vec![Style::default().style(line)];
        }

        spans.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

        let mut styled: Vec<owo_colors::Styled<&'s str>> = Vec::new();
        let mut pos = 0usize;
        for (mut start, end, capture_index) in spans {
            if end <= pos {
                continue;
            }
            if start < pos {
                start = pos;
            }

            if pos < start {
                styled.push(Style::default().style(&line[pos..start]));
            }

            let capture_name = query.capture_names()[capture_index];
            let rgb = super::colors::color_for_capture(&Base16Theme::dracula(), capture_name);
            let style = Style::new().truecolor(rgb.r, rgb.g, rgb.b);
            styled.push(style.style(&line[start..end]));
            pos = end;
        }

        if pos < line.len() {
            styled.push(Style::default().style(&line[pos..]));
        }

        styled
    }
}
