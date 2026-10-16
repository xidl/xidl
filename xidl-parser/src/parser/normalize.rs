use std::borrow::Cow;
use std::ops::Range;

use crate::lint::AnnotationDefinition;

pub fn source(text: &str) -> Cow<'_, str> {
    SourceNormalization {
        text,
        edits: Vec::new(),
    }
    .apply()
}

struct SourceNormalization<'a> {
    text: &'a str,
    edits: Vec<(Range<usize>, String)>,
}

impl<'a> SourceNormalization<'a> {
    fn apply(mut self) -> Cow<'a, str> {
        let bytes = self.text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'"' | b'\'' => index = self.quoted_end(index),
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    index = self.text[index + 2..]
                        .find("*/")
                        .map_or(bytes.len(), |end| index + 4 + end);
                }
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    if bytes.get(index + 2) == Some(&b'@') {
                        index += 2;
                    } else {
                        index = self.text[index..]
                            .find('\n')
                            .map_or(bytes.len(), |end| index + end);
                    }
                }
                b'@' => index = self.annotation(index),
                _ => index += 1,
            }
        }
        if self.edits.is_empty() {
            return Cow::Borrowed(self.text);
        }
        let mut out = self.text.to_owned();
        for (range, replacement) in self.edits.into_iter().rev() {
            out.replace_range(range, &replacement);
        }
        Cow::Owned(out)
    }

    fn quoted_end(&self, start: usize) -> usize {
        let bytes = self.text.as_bytes();
        let mut index = start + 1;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index = (index + 2).min(bytes.len());
            } else if bytes[index] == bytes[start] {
                return index + 1;
            } else {
                index += 1;
            }
        }
        index
    }

    fn annotation(&mut self, start: usize) -> usize {
        let bytes = self.text.as_bytes();
        let name_start = self.skip_trivia(start + 1);
        let mut end = name_start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'-' | b':'))
        {
            end += 1;
        }
        let name = &self.text[name_start..end];
        if let Some(definition) = AnnotationDefinition::lookup(name) {
            if definition.name != name {
                self.edits
                    .push((name_start..end, definition.name.to_string()));
            }
        } else if AnnotationDefinition::is_passthrough(name) && name.contains('-') {
            self.edits.push((name_start..end, name.replace('-', "_")));
        }
        let arguments = self.skip_trivia(end);
        if bytes.get(arguments) == Some(&b'(') {
            if let Some(close) = self.masked_end(arguments) {
                let replacement = bytes[arguments + 1..close]
                    .iter()
                    .map(|byte| {
                        if matches!(byte, b'\n' | b'\r') {
                            *byte as char
                        } else {
                            ' '
                        }
                    })
                    .collect();
                self.edits.push((arguments + 1..close, replacement));
                return close + 1;
            }
        }
        end
    }

    fn skip_trivia(&self, mut index: usize) -> usize {
        let bytes = self.text.as_bytes();
        loop {
            while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            if bytes.get(index..index + 2) == Some(b"/*") {
                index = self.text[index + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |end| index + 4 + end);
            } else {
                return index;
            }
        }
    }

    fn masked_end(&self, start: usize) -> Option<usize> {
        let bytes = self.text.as_bytes();
        let mut index = start + 1;
        let mut depth = 1;
        let mut bracket = false;
        while index < bytes.len() {
            match bytes[index] {
                b'"' | b'\'' => {
                    index = self.quoted_end(index);
                    continue;
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    index = self.skip_trivia(index);
                    continue;
                }
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    index = self.text[index..]
                        .find('\n')
                        .map_or(bytes.len(), |end| index + end);
                    continue;
                }
                b'[' | b']' => bracket = true,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return bracket.then_some(index);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        None
    }
}
