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

pub use render::{CppRender, CppRenderOutput, CppRenderer};

use crate::error::IdlcResult;
use crate::generate::cpp::util::{cpp_header, cpp_source_header};
use crate::generate::GeneratedFile;
use std::path::Path;
use xidl_parser::hir;

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let header_name = format!("{base}.h");

    let renderer = CppRenderer::new()?;
    let mut output = spec.render(&renderer)?;

    let mut header_chunks = Vec::new();
    header_chunks.push(cpp_header());
    header_chunks.append(&mut output.header);

    let mut source_chunks = Vec::new();
    source_chunks.push(cpp_source_header(&header_name));
    source_chunks.append(&mut output.source);

    Ok(vec![
        GeneratedFile {
            filename: header_name.clone(),
            filecontent: header_chunks.join("\n"),
        },
        GeneratedFile {
            filename: format!("{base}.cpp"),
            filecontent: source_chunks.join("\n"),
        },
    ])
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
