use super::Markdown;
use crate::symbols::{SymbolScope, Symbols};
use pulldown_cmark::{BrokenLink, Event, LinkType, Parser, Tag, TagEnd};

impl Markdown<'_> {
    /// Expands only undefined reference links, retaining the author's Markdown bytes.
    pub fn link_symbols(source: &str, symbols: &Symbols<'_>, scope: SymbolScope<'_>) -> String {
        let authored = Parser::new_ext(source, Self::options())
            .into_offset_iter()
            .filter_map(|(event, range)| {
                matches!(event, Event::Start(Tag::Link { .. } | Tag::Image { .. })).then_some(range)
            })
            .collect::<Vec<_>>();
        let mut callback = |link: BrokenLink<'_>| {
            // Resolving a nested label can otherwise disable its authored outer link.
            if authored
                .iter()
                .any(|range| range.contains(&link.span.start))
            {
                return None;
            }
            symbols
                .resolve(&scope, &link.reference)
                .map(|url| (url.to_owned().into(), "".into()))
        };
        let parser =
            Parser::new_with_broken_link_callback(source, Self::options(), Some(&mut callback));
        let mut output = String::new();
        let mut cursor = 0;
        let mut image_depth = 0;
        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Image { .. }) => image_depth += 1,
                Event::End(TagEnd::Image) => image_depth -= 1,
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    ..
                }) if image_depth == 0
                    && matches!(
                        link_type,
                        LinkType::ReferenceUnknown
                            | LinkType::CollapsedUnknown
                            | LinkType::ShortcutUnknown
                    ) =>
                {
                    let original = &source[range.clone()];
                    let label = match link_type {
                        LinkType::ReferenceUnknown => original
                            .rsplit_once('[')
                            .map_or(original, |(label, _)| label.trim_end()),
                        _ => original,
                    };
                    output.push_str(&source[cursor..range.start]);
                    output.push_str(label);
                    output.push_str("(<");
                    output.push_str(&dest_url);
                    output.push_str(">)");
                    // The collapsed-reference source span ends before its trailing [].
                    cursor = range.end;
                    if link_type == LinkType::CollapsedUnknown {
                        cursor += 2;
                    }
                }
                _ => {}
            }
        }
        output.push_str(&source[cursor..]);
        output
    }
}
