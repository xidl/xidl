use super::super::model::{MethodModel, TsHttpBlocks};
use crate::error::IdlcResult;
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::contexts::{RequestContext, RequestZodContext};

pub(super) fn render_request_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    _module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.request_name.as_ref() else {
        return Ok(());
    };
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.request_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: name.clone(), // Template adds 'Schema'
            name: name.to_string(),
            params: method.request_fields.clone(),
        },
    )?);
    Ok(())
}

pub(super) fn render_response_types(
    out: &mut TsHttpBlocks,
    method: &MethodModel,
    _module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<()> {
    let Some(name) = method.response_name.as_ref() else {
        return Ok(());
    };
    out.types.push(renderer.render_template(
        "http/request.d.ts.j2",
        &RequestContext {
            name: name.to_string(),
            params: method.response_fields.clone(),
            doc: Vec::new(),
        },
    )?);
    out.zod.push(renderer.render_template(
        "http/request.zod.ts.j2",
        &RequestZodContext {
            schema_name: name.clone(), // Template adds 'Schema'
            name: name.to_string(),
            params: method.response_fields.clone(),
        },
    )?);
    Ok(())
}
