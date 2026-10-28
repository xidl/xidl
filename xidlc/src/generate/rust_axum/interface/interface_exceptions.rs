use super::interface_types::{header_item_is_primitive, header_item_is_string};
use super::{ExceptionMemberContext, RaisesContext, RenderEnv};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::transport::{TransportDirection, TransportTracker};
use convert_case::Casing;
use xidl_parser::rest_hir::HttpOperation;

impl RaisesContext {
    /// Projects `raises(...)` entries into template contexts, pairing each
    /// operation-level ref (order preserved from the IDL) with its declared
    /// exception.
    pub(super) fn for_operation(
        http_op: &HttpOperation,
        env: RenderEnv<'_>,
        transport: &mut TransportTracker,
    ) -> IdlcResult<Vec<RaisesContext>> {
        if http_op.meta.raises.is_empty() {
            return Ok(Vec::new());
        }
        let document = env.renderer.rest_hir()?;
        let names = Self::variant_names(http_op, &document)?;
        let mut out = Vec::new();
        for (refer, variant) in http_op.meta.raises.iter().zip(names) {
            let exception = document
                .document
                .exceptions
                .iter()
                .find(|e| e.ident == refer.ident && e.module_path == refer.module_path)
                .ok_or_else(|| {
                    IdlcError::rpc(format!(
                        "raises('{}') does not resolve to a declared exception",
                        refer.ident
                    ))
                })?;
            let member_env = env.in_declaration(&exception.module_path);
            let members = |source: &Vec<xidl_parser::rest_hir::HttpExceptionMember>| {
                source
                    .iter()
                    .map(|member| {
                        let item = match &member.ty {
                            xidl_parser::hir::TypeSpec::SequenceType(sequence) => {
                                sequence.ty.as_ref()
                            }
                            ty => ty,
                        };
                        let item_ty = transport.map_type(
                            item,
                            TransportDirection::Out,
                            member_env.scope(),
                        )?;
                        let wire_ty = transport.map_type(
                            &member.ty,
                            TransportDirection::Out,
                            member_env.scope(),
                        )?;
                        Ok(ExceptionMemberContext {
                            field: rust_ident(&member.field),
                            wire_name: member.wire_name.clone(),
                            ty: if member.is_multi {
                                format!("Vec<{item_ty}>")
                            } else if member.optional {
                                format!("Option<{wire_ty}>")
                            } else {
                                wire_ty
                            },
                            is_multi: member.is_multi,
                            item_ty,
                            item_is_string: header_item_is_string(&member.ty),
                            item_is_primitive: header_item_is_primitive(&member.ty),
                            optional: member.optional,
                            maximum: member_env.scope().sequence_maximum(&member.ty),
                        })
                    })
                    .collect::<IdlcResult<Vec<_>>>()
            };
            let mut members = members;
            let headers = members(&exception.headers)?;
            let cookies = members(&exception.cookies)?;
            let mut name = refer.module_path.clone();
            name.push(refer.ident.clone());
            let wire_ty = transport.map_type(
                &xidl_parser::hir::TypeSpec::ScopedName(xidl_parser::hir::ScopedName {
                    is_root: true,
                    name,
                }),
                TransportDirection::Out,
                env.scope(),
            )?;
            out.push(RaisesContext {
                variant,
                ty: env.relative_type_path(&refer.module_path, &refer.ident),
                wire_ty,
                status: exception.status,
                headers,
                cookies,
                has_body: !exception.body.is_empty(),
                deprecated: exception.deprecated.is_some(),
            });
        }
        Ok(out)
    }

    /// Names shared by ordinary errors and independent adapter match arms.
    pub(crate) fn variant_names(
        http_op: &HttpOperation,
        document: &xidl_parser::rest_hir::RestHirDocument,
    ) -> IdlcResult<Vec<String>> {
        let variants = http_op
            .meta
            .raises
            .iter()
            .map(|refer| rust_ident(&refer.ident).to_case(convert_case::Case::Pascal))
            .collect::<Vec<_>>();
        let qualify_variants = variants.iter().any(|name| name == "Framework")
            || variants
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != variants.len();
        http_op
            .meta
            .raises
            .iter()
            .map(|refer| {
                let exception = document
                    .document
                    .exceptions
                    .iter()
                    .find(|value| {
                        value.ident == refer.ident && value.module_path == refer.module_path
                    })
                    .ok_or_else(|| {
                        IdlcError::rpc(format!(
                            "raises('{}') does not resolve to a declared exception",
                            refer.ident
                        ))
                    })?;
                let name = rust_ident(&exception.ident).to_case(convert_case::Case::Pascal);
                Ok(if qualify_variants {
                    format!("Http{}{name}", exception.status)
                } else {
                    name
                })
            })
            .collect()
    }
}
