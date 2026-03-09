pub mod filter;
#[cfg(test)]
mod tests;

pub use filter::{
    clang_format_filter, format_timestamp_filter, rust_format_filter, to_case,
    typescript_format_filter,
};

pub fn strip_template_padding(input: String) -> String {
    let mut out = String::with_capacity(input.len());
    for (idx, line) in input.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("    ") {
            out.push_str(stripped);
        } else {
            out.push_str(line);
        }
    }
    out
}
