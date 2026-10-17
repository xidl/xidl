use super::interface_types::{header_item_is_primitive, header_item_is_string};
use super::{ExceptionMemberContext, RaisesContext, RenderEnv};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::rust_ident;
use convert_case::Casing;
use xidl_parser::rest_hir::HttpOperation;

impl RaisesContext {
    /// Projects `raises(...)` entries into template contexts, pairing each
    /// operation-level ref (order preserved from the IDL) with its declared
    /// exception.
    pub(super) fn for_operation(
        http_op: &HttpOperation,
        env: RenderEnv<'_>,
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
                    .map(|member| ExceptionMemberContext {
                        field: rust_ident(&member.field),
                        wire_name: member.wire_name.clone(),
                        ty: if member.is_multi {
                            format!("Vec<{}>", member_env.header_item_ty(&member.ty))
                        } else if member.optional {
                            format!("Option<{}>", member_env.axum_type(&member.ty))
                        } else {
                            member_env.axum_type(&member.ty)
                        },
                        is_multi: member.is_multi,
                        item_ty: member_env.header_item_ty(&member.ty),
                        item_is_string: header_item_is_string(&member.ty),
                        item_is_primitive: header_item_is_primitive(&member.ty),
                        optional: member.optional,
                    })
                    .collect()
            };
            out.push(RaisesContext {
                variant,
                ty: env.relative_type_path(&refer.module_path, &refer.ident),
                status: exception.status,
                headers: members(&exception.headers),
                cookies: members(&exception.cookies),
                has_body: !exception.body.is_empty(),
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
