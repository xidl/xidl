use crate::highlight::theme::{Base16Theme, Rgb};
use std::collections::HashMap;

pub struct CaptureColors {
    map: HashMap<String, String>,
    default_color: String,
}

impl CaptureColors {
    pub fn new(palette: &Base16Theme, names: &[&'static str]) -> Self {
        let mut map = HashMap::new();
        for name in names {
            let rgb = color_for_capture(palette, name);
            map.insert((*name).to_string(), ansi_fg(rgb));
        }
        let default_color = ansi_fg(palette.base05);
        Self { map, default_color }
    }

    pub fn color_for(&self, name: &str) -> &str {
        self.map
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(self.default_color.as_str())
    }

    pub fn default_color(&self) -> &str {
        self.default_color.as_str()
    }
}

fn ansi_fg(rgb: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
}

fn color_for_capture(theme: &Base16Theme, name: &str) -> Rgb {
    let (group, suffix) = capture_group(name);
    match group {
        "comment" => match suffix {
            Some("documentation") => theme.base04,
            Some("error") => theme.base08,
            Some("warning") => theme.base09,
            Some("todo") => theme.base0e,
            Some("note") => theme.base0d,
            _ => theme.base03,
        },
        "string" => match suffix {
            Some("documentation") => theme.base04,
            Some("escape") => theme.base0c,
            Some("regexp") => theme.base0f,
            Some("special") => theme.base0d,
            Some(s) if s.starts_with("special.") => theme.base0d,
            _ => theme.base0b,
        },
        "character" => match suffix {
            Some("special") => theme.base0a,
            _ => theme.base0b,
        },
        "number" => match suffix {
            Some("float") => theme.base09,
            _ => theme.base09,
        },
        "boolean" => theme.base09,
        "keyword" => theme.base0e,
        "type" => match suffix {
            Some("builtin") => theme.base0a,
            Some("definition") => theme.base0d,
            _ => theme.base0a,
        },
        "variable" => match suffix {
            Some("builtin") => theme.base09,
            Some("parameter") | Some("parameter.builtin") => theme.base08,
            Some("member") => theme.base08,
            _ => theme.base08,
        },
        "constant" => match suffix {
            Some("builtin") => theme.base09,
            Some("macro") => theme.base0c,
            _ => theme.base09,
        },
        "function" => match suffix {
            Some("builtin") => theme.base0d,
            Some("call") => theme.base0d,
            Some("macro") => theme.base0c,
            Some("method") | Some("method.call") => theme.base0d,
            _ => theme.base0d,
        },
        "attribute" => match suffix {
            Some("builtin") => theme.base0e,
            _ => theme.base0e,
        },
        "module" => match suffix {
            Some("builtin") => theme.base0d,
            _ => theme.base0d,
        },
        "property" => theme.base0c,
        "operator" => theme.base07,
        "punctuation" => match suffix {
            Some("delimiter") => theme.base05,
            Some("bracket") => theme.base05,
            Some("special") => theme.base02,
            _ => theme.base05,
        },
        "markup" => match suffix {
            Some("strong") => theme.base0a,
            Some("italic") => theme.base0d,
            Some("strikethrough") => theme.base01,
            Some("underline") => theme.base06,
            Some("heading") => theme.base06,
            Some(s) if s.starts_with("heading.") => theme.base06,
            Some("quote") => theme.base03,
            Some("math") => theme.base0c,
            Some("link") | Some("link.label") | Some("link.url") => theme.base0d,
            Some("raw") => theme.base02,
            Some("raw.block") => theme.base00,
            Some(s) if s.starts_with("list") => theme.base05,
            _ => theme.base05,
        },
        "diff" => match suffix {
            Some("plus") => theme.base0b,
            Some("minus") => theme.base08,
            Some("delta") => theme.base0a,
            _ => theme.base0a,
        },
        "tag" => match suffix {
            Some("attribute") => theme.base0c,
            Some("delimiter") => theme.base05,
            Some("builtin") => theme.base0d,
            _ => theme.base0d,
        },
        "label" => theme.base0f,
        _ => theme.base05,
    }
}

fn capture_group(name: &str) -> (&str, Option<&str>) {
    let mut parts = name.splitn(2, '.');
    let group = parts.next().unwrap_or(name);
    let suffix = parts.next();
    (group, suffix)
}
