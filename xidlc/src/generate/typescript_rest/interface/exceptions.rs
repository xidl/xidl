use super::super::model::{TsExceptionFieldContext, TsExceptionMemberContext, TsRaisesContext};
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::type_expr::ts_type_for_type_spec;
use xidl_parser::rest_hir::HttpOperation;

impl TsRaisesContext {
    /// Pairs each `raises(...)` ref with its projected exception for the
    /// typescript-rest templates.
    pub(super) fn for_operation(
        op: &HttpOperation,
        exceptions: &[xidl_parser::rest_hir::HttpException],
        module_path: &[String],
    ) -> Vec<TsRaisesContext> {
        op.meta
            .raises
            .iter()
            .filter_map(|refer| {
                let exception = exceptions
                    .iter()
                    .find(|e| e.ident == refer.ident && e.module_path == refer.module_path)?;
                let member =
                    |m: &xidl_parser::rest_hir::HttpExceptionMember| TsExceptionMemberContext {
                        field: m.field.clone(),
                        wire_name: m.wire_name.clone(),
                        ty: ts_type_for_type_spec(&m.ty, module_path, TypeRefTarget::Client),
                        is_multi: m.is_multi,
                        optional: m.optional,
                    };
                let field =
                    |f: &xidl_parser::rest_hir::HttpExceptionField| TsExceptionFieldContext {
                        field: f.field.clone(),
                        ty: ts_type_for_type_spec(&f.ty, module_path, TypeRefTarget::Client),
                    };
                Some(TsRaisesContext {
                    ident: exception.ident.clone(),
                    status: exception.status,
                    has_body: !exception.body.is_empty(),
                    headers: exception.headers.iter().map(member).collect(),
                    cookies: exception.cookies.iter().map(member).collect(),
                    body: exception.body.iter().map(field).collect(),
                })
            })
            .collect()
    }
}
