use similar::{ChangeTag, TextDiff};

pub(super) struct ComparisonLines {
    pub before: String,
    pub after: String,
}

impl ComparisonLines {
    pub fn between(before: &str, after: &str) -> Self {
        let mut removed = Vec::new();
        let mut inserted = Vec::new();
        for change in TextDiff::from_lines(before, after).iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    if let Some(line) = change.old_index() {
                        removed.push((line + 1).to_string());
                    }
                }
                ChangeTag::Insert => {
                    if let Some(line) = change.new_index() {
                        inserted.push((line + 1).to_string());
                    }
                }
                ChangeTag::Equal => {}
            }
        }
        Self {
            before: removed.join(","),
            after: inserted.join(","),
        }
    }
}
