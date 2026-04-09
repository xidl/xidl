pub fn extract(source: &[u8], start: usize) -> Option<String> {
    if start == 0 {
        return None;
    }

    let mut line_end = if source[start - 1] == b'\n' {
        start - 1
    } else {
        start
    };
    let mut lines = Vec::new();
    let mut first = true;

    loop {
        let line_start = line_start(source, line_end);
        let line = trim_line_end(&source[line_start..line_end]);

        if line.iter().all(|b| b.is_ascii_whitespace()) {
            if first && line_start > 0 {
                line_end = line_start - 1;
                first = false;
                continue;
            }
            break;
        }

        first = false;
        let trimmed = trim_ascii_start(line);
        if !trimmed.starts_with(b"///") {
            break;
        }

        lines.push(doc_line(trimmed));
        if line_start == 0 {
            break;
        }
        line_end = line_start - 1;
    }

    if lines.is_empty() {
        None
    } else {
        lines.reverse();
        Some(lines.join("\n"))
    }
}

fn line_start(source: &[u8], line_end: usize) -> usize {
    let mut index = line_end;
    while index > 0 && source[index - 1] != b'\n' {
        index -= 1;
    }
    index
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn trim_ascii_start(line: &[u8]) -> &[u8] {
    let offset = line
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(line.len());
    &line[offset..]
}

fn doc_line(line: &[u8]) -> String {
    let content = line
        .strip_prefix(b"/// ")
        .or_else(|| line.strip_prefix(b"///"))
        .unwrap_or_default();
    String::from_utf8_lossy(content).into_owned()
}
