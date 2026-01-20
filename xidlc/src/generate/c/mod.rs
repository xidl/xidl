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
mod xcdr;

pub use render::{CRender, CRenderOutput, CRenderer};

use crate::error::IdlcResult;
use crate::generate::GeneratedFile;
use serde_json::json;
use std::path::Path;
use xidl_parser::hir;

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let filename = format!("{base}.h");
    let xcdr_header_name = format!("{base}_xcdr.h");

    let renderer = CRenderer::new()?;
    let output = spec.render(&renderer)?;

    let header = renderer.render_template(
        "spec.h.j2",
        &json!({
            "definitions": output.header,
            "filename": filename,
        }),
    )?;
    let source = renderer.render_template(
        "spec.c.j2",
        &json!({
            "header_name": filename,
            "definitions": output.source,
        }),
    )?;
    let xcdr_header = renderer.render_template(
        "spec.h.j2",
        &json!({
            "definitions": output.xcdr_header,
            "filename": xcdr_header_name,
        }),
    )?;
    let xcdr_source = renderer.render_template(
        "spec.c.j2",
        &json!({
            "header_name": xcdr_header_name,
            "definitions": output.xcdr_source,
        }),
    )?;

    Ok(vec![
        GeneratedFile {
            filename: filename.clone(),
            filecontent: header,
        },
        GeneratedFile {
            filename: format!("{base}.c"),
            filecontent: source,
        },
        GeneratedFile {
            filename: xcdr_header_name.clone(),
            filecontent: xcdr_header,
        },
        GeneratedFile {
            filename: format!("{base}_xcdr.c"),
            filecontent: xcdr_source,
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
