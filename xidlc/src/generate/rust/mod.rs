mod bitmask_dcl;
mod bitset_dcl;
mod const_dcl;
mod constr_type;
mod definition;
mod enum_dcl;
mod exception_dcl;
mod interface_dcl;
mod render;
mod spec;
mod struct_dcl;
mod type_dcl;
mod union_def;
mod util;

pub use render::{RustRender, RustRenderOutput, RustRenderer};

use crate::error::IdlcResult;
use crate::generate::rust::util::rust_header;
use crate::generate::GeneratedFile;
use std::path::Path;
use xidl_parser::hir;

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let filename = format!("{base}.rs");

    let renderer = RustRenderer::new()?;
    let mut output = spec.render(&renderer)?;

    let mut chunks = Vec::new();
    chunks.push(rust_header());
    chunks.append(&mut output.source);

    Ok(vec![GeneratedFile {
        filename,
        filecontent: chunks.join("\n"),
    }])
}

pub fn serve_jsonrpc<R: std::io::BufRead, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IdlcResult<()> {
    crate::generate::jsonrpc::serve_generate(reader, writer, |spec, input| {
        let input = input.ok_or_else(|| crate::error::IdlcError::rpc("missing input path"))?;
        generate(spec, Path::new(input))
    })
}
