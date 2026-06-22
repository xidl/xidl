use super::{TransportTypeDef, TypeRegistry, scoped_key};
use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;

pub(crate) fn encode_expr(expr: &str, ty: &hir::TypeSpec, r: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, r)
}

pub(crate) fn decode_expr(expr: &str, ty: &hir::TypeSpec, r: &TypeRegistry) -> IdlcResult<String> {
    convert_expr(expr, ty, r)
}

fn convert_expr(expr: &str, ty: &hir::TypeSpec, registry: &TypeRegistry) -> IdlcResult<String> {
    Ok(match ty {
        hir::TypeSpec::SequenceType(seq) => {
            let inner = convert_expr("value", &seq.ty, registry)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|value| {inner}).collect()")
            }
        }
        hir::TypeSpec::MapType(map) => {
            let inner = convert_expr("value", &map.value, registry)?;
            if inner == "value" {
                format!("{expr}.into_iter().collect()")
            } else {
                format!("{expr}.into_iter().map(|(key, value)| (key, {inner})).collect()")
            }
        }
        hir::TypeSpec::ScopedName(value) => {
            let name = scoped_key(value);
            match registry.get(&name) {
                Some(TransportTypeDef::Struct(_)) | Some(TransportTypeDef::Enum(_)) => {
                    format!("{expr}.into()")
                }
                Some(TransportTypeDef::Typedef(def)) => match &def.ty {
                    hir::TypedefType::TypeSpec(inner) => convert_expr(expr, inner, registry)?,
                    hir::TypedefType::ConstrTypeDcl(_) => {
                        return Err(IdlcError::rpc(format!(
                            "unsupported inline typedef transport for '{}'",
                            name
                        )));
                    }
                },
                None => expr.to_string(),
            }
        }
        _ => expr.to_string(),
    })
}
