//! Collect HTTP signature shapes and their existing wire bindings.

use super::*;

impl SurfaceCollector {
    pub(super) fn operation(
        interface: &str,
        scope: &[String],
        http: &HttpOperation,
    ) -> IdlcResult<Operation> {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for parameter in &http.signature.params {
            let mut member = Member {
                name: parameter.name.clone(),
                wire: parameter.name.clone(),
                key: parameter.name.clone(),
                flatten: false,
                value: Some(TypeUse {
                    ty: parameter.ty.clone(),
                    scope: scope.to_vec(),
                    declarator: hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
                        parameter.name.clone(),
                    )),
                    optional: parameter.is_optional,
                    // A parameter default would need rest_hir to carry it; the
                    // issue covers struct members, where the fact already lives.
                    defaulted: false,
                    representation: ValueRepresentation::Model,
                }),
            };
            if matches!(
                parameter.direction,
                HttpSignatureParamDirection::In | HttpSignatureParamDirection::InOut
            ) {
                let (wire, location) = Self::binding(http, &parameter.name)?;
                member.wire = wire;
                member.key = format!("{location}:{}", member.wire);
                let mut input = member.clone();
                if location == "body"
                    && http.http.request.body.shape.byte_parameter()
                        == Some(parameter.name.as_str())
                {
                    if let Some(value) = &mut input.value {
                        value.representation = ValueRepresentation::ByteBuffer;
                    }
                }
                inputs.push(input);
            }
            if matches!(
                parameter.direction,
                HttpSignatureParamDirection::Out | HttpSignatureParamDirection::InOut
            ) {
                member.key = parameter.name.clone();
                outputs.push(member);
            }
        }
        if let Some(ty) = &http.signature.return_type {
            outputs.insert(
                0,
                Member {
                    name: "return".into(),
                    wire: "return".into(),
                    key: "return".into(),
                    flatten: false,
                    value: Some(TypeUse {
                        ty: ty.clone(),
                        scope: scope.to_vec(),
                        declarator: hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
                            "return".into(),
                        )),
                        optional: false,
                        defaulted: false,
                        representation: ValueRepresentation::Model,
                    }),
                },
            );
        }
        Ok(Operation {
            interface: interface.to_string(),
            name: http.meta.name.clone(),
            scope: scope.to_vec(),
            inputs,
            outputs,
            raises: http
                .meta
                .raises
                .iter()
                .map(|reference| {
                    let modules = reference
                        .module_path
                        .iter()
                        .skip(1)
                        .cloned()
                        .collect::<Vec<_>>();
                    Self::join(&modules, &reference.ident)
                })
                .collect(),
            http: http.clone(),
        })
    }

    fn binding(http: &HttpOperation, name: &str) -> IdlcResult<(String, &'static str)> {
        for (bindings, location) in [
            (&http.http.request.path, "path"),
            (&http.http.request.query, "query"),
            (&http.http.request.header, "header"),
            (&http.http.request.cookie, "cookie"),
        ] {
            if let Some(binding) = bindings.iter().find(|binding| binding.source_param == name) {
                return Ok((binding.wire_name.clone(), location));
            }
        }
        let body = match &http.http.request.body.shape {
            HttpRequestBodyShape::SingleValue {
                source_param,
                wire_name,
                ..
            }
            | HttpRequestBodyShape::Stream {
                source_param,
                wire_name,
                ..
            } if source_param == name => Some(wire_name.clone()),
            HttpRequestBodyShape::Object { fields } => fields
                .iter()
                .find(|field| field.source_param == name)
                .map(|field| field.field_name.clone()),
            _ => None,
        };
        body.map(|wire| (wire, "body")).ok_or_else(|| {
            IdlcError::fmt(format!(
                "version adapters: operation {} has no HTTP binding for parameter {name}",
                http.meta.name
            ))
        })
    }
}
