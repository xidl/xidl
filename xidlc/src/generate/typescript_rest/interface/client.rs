use super::super::model::ClientClassContext;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use serde::Serialize;
use xidl_parser::hir::VersionParameter;

#[derive(Serialize)]
struct VersionClientContext<'a> {
    api_version: &'a str,
    client_name: &'a str,
    parameter: &'a VersionParameter,
}

pub(super) fn render(
    renderer: &TypescriptRenderer,
    context: &ClientClassContext,
    version: Option<(&VersionParameter, &str)>,
) -> IdlcResult<String> {
    let mut output = renderer.render_template("http/client_class.ts.j2", context)?;
    let Some((parameter, value)) = version else {
        return Ok(output);
    };
    let snippet_context = VersionClientContext {
        api_version: value,
        client_name: &context.client_name,
        parameter,
    };
    let static_block =
        renderer.render_template("http/version_client_static.ts.j2", &snippet_context)?;
    let marker = format!("export class {}Client {{", context.client_name);
    let replacement = format!("{marker}\n{static_block}");
    if !output.contains(&marker) {
        return Err(IdlcError::template(format!(
            "typescript client template is missing class marker '{marker}'"
        )));
    }
    output = output.replacen(&marker, &replacement, 1);
    let pin = renderer.render_template("http/version_client_pin.ts.j2", &snippet_context)?;
    let pin = pin
        .lines()
        .map(|line| format!("                {line}\n"))
        .collect::<String>();
    let mut rendered = String::with_capacity(output.len() + pin.len() * 4);
    for line in output.split_inclusive('\n') {
        rendered.push_str(line);
        if line.contains("applyClientAuth(") {
            rendered.push_str(&pin);
        }
    }
    let normalized = rendered
        .lines()
        .map(|line| if line.trim().is_empty() { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!("{normalized}\n"))
}
