use tree_sitter::{Query, QueryCursor, StreamingIterator};

use super::{InsertKind, QueryActions, add_indent, push_action};

pub(super) fn collect_actions(
    source: &str,
    root: tree_sitter::Node<'_>,
    query: &Query,
) -> QueryActions {
    let mut actions = QueryActions::default();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures {
            let name = &query.capture_names()[capture.index as usize];
            let node = capture.node;
            match *name {
                "append-space" => push_action(
                    &mut actions.append,
                    node.end_byte(),
                    InsertKind::AppendSpace,
                ),
                "prepend-space" => push_action(
                    &mut actions.prepend,
                    node.start_byte(),
                    InsertKind::PrependSpace,
                ),
                "append-newline" => push_action(
                    &mut actions.append,
                    node.end_byte(),
                    InsertKind::AppendNewline,
                ),
                "prepend-newline" => push_action(
                    &mut actions.prepend,
                    node.start_byte(),
                    InsertKind::PrependNewline,
                ),
                "add-ident" => add_indent(&mut actions.indent_post, node.end_byte(), 1),
                "dec-ident" => add_indent(&mut actions.indent_pre, node.start_byte(), -1),
                "comment" => super::mark_comment(&mut actions, node.start_byte(), node.end_byte()),
                _ => {}
            }
        }
    }
    actions
}
