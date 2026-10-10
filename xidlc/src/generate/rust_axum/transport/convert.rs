use super::super::scope::TypeScope;
use super::TransportTypeDef;
use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;

pub(crate) fn encode_expr(
    expr: &str,
    ty: &hir::TypeSpec,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    convert_expr(expr, ty, scope)
}

pub(crate) fn decode_expr(
    expr: &str,
    ty: &hir::TypeSpec,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    convert_expr(expr, ty, scope)
}

fn convert_expr(expr: &str, ty: &hir::TypeSpec, scope: TypeScope<'_>) -> IdlcResult<String> {
    Ok(match ty {
        hir::TypeSpec::SequenceType(seq) => {
            let inner = convert_expr("value", &seq.ty, scope)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|value| {inner}).collect()")
            }
        }
        hir::TypeSpec::MapType(map) => {
            let inner = convert_expr("value", &map.value, scope)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|(key, value)| (key, {inner})).collect()")
            }
        }
        hir::TypeSpec::ScopedName(value) => {
            let key = scope.resolve(value);
            let Some(key) = key else {
                return Ok(expr.to_string());
            };
            match scope.registry.get(key.as_str()) {
                Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
                    format!("{expr}.into()")
                }
                Some(TransportTypeDef::Typedef(def)) => match &def.ty {
                    hir::TypedefType::TypeSpec(inner) => {
                        let mut parent = key.split("::").map(str::to_string).collect::<Vec<_>>();
                        parent.pop();
                        convert_expr(expr, inner, scope.in_declaration(&parent))?
                    }
                    hir::TypedefType::ConstrTypeDcl(_) => {
                        return Err(IdlcError::rpc(format!(
                            "unsupported inline typedef transport for '{key}'"
                        )));
                    }
                },
                Some(TransportTypeDef::Public) | None => expr.to_string(),
            }
        }
        _ => expr.to_string(),
    })
}
