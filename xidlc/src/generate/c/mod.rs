mod bitmask_dcl;
mod bitset_dcl;
mod const_dcl;
mod constr_type;
mod definition;
mod enum_dcl;
mod render;
mod spec;
mod struct_dcl;
mod union_def;
mod util;

pub use render::{CRender, CRenderer};

use crate::error::IdlcResult;
use crate::generate::c::util::c_header;
use crate::generate::GeneratedFile;
use std::path::Path;
use xidl_parser::hir;

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let filename = format!("{}.h", crate::generate::to_snake_case(stem));

    let renderer = CRenderer::new()?;
    let mut chunks = Vec::new();
    chunks.push(c_header());
    chunks.extend(spec.render(&renderer)?);

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
