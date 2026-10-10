use super::interface_types::render_scoped_name;
use super::interface_types::{
    axum_type, header_item_is_primitive, header_item_is_string, header_item_ty,
};
use super::{ExceptionMemberContext, RaisesContext, RenderEnv};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::rust_ident;
use convert_case::Casing;
use xidl_parser::hir;
use xidl_parser::rest_hir::HttpOperation;

impl RaisesContext {
    /// Projects `raises(...)` entries into template contexts, pairing each
    /// operation-level ref (order preserved from the IDL) with its declared
    /// exception.
    pub(super) fn for_operation(
        op: &hir::OpDcl,
        http_op: &HttpOperation,
        env: RenderEnv<'_>,
    ) -> IdlcResult<Vec<RaisesContext>> {
        if http_op.meta.raises.is_empty() {
            return Ok(Vec::new());
        }
        let document = env.renderer.rest_hir()?;
        let scoped_names = op
            .raises
            .as_ref()
            .map(|raises| raises.0.as_slice())
            .unwrap_or_default();
        let mut out = Vec::new();
        for (refer, scoped) in http_op.meta.raises.iter().zip(scoped_names) {
            let exception = document
                .document
                .exceptions
                .iter()
                .find(|e| e.ident == refer.ident && e.module_path == refer.module_path)
                .ok_or_else(|| {
                    IdlcError::rpc(format!(
                        "raises('{}') does not resolve to a declared exception",
                        scoped.name.join("::")
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
                variant: rust_ident(&exception.ident).to_case(convert_case::Case::Pascal),
                ty: render_scoped_name(scoped),
                status: exception.status,
                headers: members(&exception.headers),
                cookies: members(&exception.cookies),
                has_body: !exception.body.is_empty(),
            });
        }
        Ok(out)
    }
}
