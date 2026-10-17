/// Raw annotation argument boundaries, retaining quotes and duplicate keys.
pub(crate) struct RawArguments;

impl RawArguments {
    /// Canonicalizes argument trivia without changing quoted text or tokens.
    pub(crate) fn canonical(raw: &str) -> String {
        let raw = Self::without_comments(raw);
        let mut out = String::new();
        let mut chars = raw.chars();
        let mut quote = None;
        let mut previous = None;
        let mut whitespace = false;
        while let Some(ch) = chars.next() {
            if let Some(current) = quote {
                out.push(ch);
                if ch == '\\' {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                } else if ch == current {
                    quote = None;
                }
            } else if ch.is_whitespace() {
                whitespace = true;
                continue;
            } else {
                let delimiter =
                    |value| matches!(value, '(' | ')' | '[' | ']' | '{' | '}' | ',' | '=');
                if whitespace && previous.is_some_and(|last| !delimiter(last) && !delimiter(ch)) {
                    out.push(' ');
                }
                out.push(ch);
                if matches!(ch, '"' | '\'') {
                    quote = Some(ch);
                }
            }
            previous = Some(ch);
            whitespace = false;
        }
        out
    }

    /// Detects array syntax while ignoring bracket characters inside strings.
    pub(crate) fn contains_array(raw: &str) -> bool {
        let mut chars = raw.chars();
        let mut quote = None;
        while let Some(ch) = chars.next() {
            if let Some(current) = quote {
                if ch == '\\' {
                    chars.next();
                } else if ch == current {
                    quote = None;
                }
            } else if matches!(ch, '"' | '\'') {
                quote = Some(ch);
            } else if matches!(ch, '[' | ']') {
                return true;
            }
        }
        false
    }

    /// Removes comment trivia while retaining quoted values verbatim.
    pub(crate) fn without_comments(raw: &str) -> String {
        let mut result = String::new();
        let mut chars = raw.chars().peekable();
        let mut quote = None;
        while let Some(ch) = chars.next() {
            if let Some(current) = quote {
                result.push(ch);
                if ch == '\\' {
                    if let Some(escaped) = chars.next() {
                        result.push(escaped);
                    }
                } else if ch == current {
                    quote = None;
                }
            } else if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                while let Some(ch) = chars.next() {
                    if ch == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
                result.push(' ');
            } else if ch == '/' && chars.peek() == Some(&'/') {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
                result.push(' ');
            } else {
                if matches!(ch, '"' | '\'') {
                    quote = Some(ch);
                }
                result.push(ch);
            }
        }
        result
    }

    /// Splits only top-level separators, leaving quoted and nested data intact.
    pub(crate) fn split(raw: &str, separator: char) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut start = 0;
        let mut quote = None;
        let mut escaped = false;
        let mut depth = 0usize;
        for (index, ch) in raw.char_indices() {
            if let Some(current) = quote {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == current {
                    quote = None;
                }
                continue;
            }
            match ch {
                '"' | '\'' => quote = Some(ch),
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => depth = depth.saturating_sub(1),
                _ if ch == separator && depth == 0 => {
                    parts.push(raw[start..index].trim());
                    start = index + ch.len_utf8();
                }
                _ => {}
            }
        }
        if !raw[start..].trim().is_empty() {
            parts.push(raw[start..].trim());
        }
        parts
    }
}
