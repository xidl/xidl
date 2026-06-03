use crate::error::IdlcResult;
use crate::generate::go_rest::{GoRestRenderer, MethodMeta, ParamMeta};
use xidl_parser::hir;

use super::definition_templates::{FormatPathReplacement, FormatPathTemplate};

pub(crate) fn render_format_path_fn(out: &mut String, method: &MethodMeta) -> IdlcResult<()> {
    let raw = method
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| "/".to_string());
    let replacements = method
        .path_params
        .iter()
        .map(|param| FormatPathReplacement {
            replacement: format!("{{{}}}", param.wire_name),
            replacement_catchall: format!("{{*{}}}", param.wire_name),
            expr: path_param_expr(param),
        })
        .collect();
    let renderer = GoRestRenderer::new()?;
    out.push_str(&renderer.render_template(
        "format_path.go.j2",
        &FormatPathTemplate {
            struct_prefix: &method.struct_prefix,
            request_struct: &method.request_struct,
            raw_path: &raw,
            trim_query_template: raw.contains("{?"),
            replacements,
        },
    )?);
    Ok(())
}

pub(crate) fn strip_interfaces(spec: hir::Specification) -> hir::Specification {
    hir::Specification(strip_defs(spec.0))
}

fn path_param_expr(param: &ParamMeta) -> String {
    let field = format!("req.{}", param.field_name);
    match param.ty.as_str() {
        "uint32" => format!("xidlgohttp.FormatUint32({field})"),
        "int32" => format!("xidlgohttp.FormatInt32({field})"),
        "uint64" => format!("xidlgohttp.FormatUint64({field})"),
        "int64" => format!("xidlgohttp.FormatInt64({field})"),
        "bool" => format!("xidlgohttp.FormatBool({field})"),
        _ => field,
    }
}

fn strip_defs(defs: Vec<hir::Definition>) -> Vec<hir::Definition> {
    let mut out = Vec::new();
    for def in defs {
        match def {
            hir::Definition::InterfaceDcl(_) => {}
            hir::Definition::ModuleDcl(mut module) => {
                module.definition = strip_defs(module.definition);
                if !module.definition.is_empty() {
                    out.push(hir::Definition::ModuleDcl(module));
                }
            }
            other => out.push(other),
        }
    }
    out
}
