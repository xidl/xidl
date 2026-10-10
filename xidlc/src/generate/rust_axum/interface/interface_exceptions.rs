use super::interface_types::{
    axum_type, header_item_is_primitive, header_item_is_string, header_item_ty,
};
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
        let mut out = Vec::new();
        for refer in &http_op.meta.raises {
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
            let members = |source: &Vec<xidl_parser::rest_hir::HttpExceptionMember>| {
                source
                    .iter()
                    .map(|member| ExceptionMemberContext {
                        field: rust_ident(&member.field),
                        wire_name: member.wire_name.clone(),
                        ty: if member.is_multi {
                            format!("Vec<{}>", header_item_ty(&member.ty))
                        } else if member.optional {
                            format!("Option<{}>", axum_type(&member.ty))
                        } else {
                            axum_type(&member.ty)
                        },
                        is_multi: member.is_multi,
                        item_ty: header_item_ty(&member.ty),
                        item_is_string: header_item_is_string(&member.ty),
                        item_is_primitive: header_item_is_primitive(&member.ty),
                        optional: member.optional,
                    })
                    .collect()
            };
            out.push(RaisesContext {
                variant: if http_op
                    .meta
                    .raises
                    .iter()
                    .filter(|entry| entry.ident == refer.ident)
                    .count()
                    > 1
                {
                    refer
                        .module_path
                        .iter()
                        .chain(std::iter::once(&refer.ident))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("_")
                        .to_case(convert_case::Case::Pascal)
                } else {
                    rust_ident(&exception.ident).to_case(convert_case::Case::Pascal)
                },
                ty: env.relative_type_path(&refer.module_path, &refer.ident),
                status: exception.status,
                headers: members(&exception.headers),
                cookies: members(&exception.cookies),
                has_body: !exception.body.is_empty(),
            });
        }
        Ok(out)
    }
}
