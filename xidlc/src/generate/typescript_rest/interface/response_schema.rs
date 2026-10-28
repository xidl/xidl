use super::super::model::MethodModel;
use super::helpers::only_direct_return;
use crate::generate::typescript::definition::contexts::ZodSchema;
use crate::generate::typescript::definition::type_expr::zod_schema_for_type_spec_with_prefix;
use xidl_parser::{hir, rest_hir::HttpOperation};

impl MethodModel {
    pub(super) fn response_value_schema(
        op: &HttpOperation,
        module_path: &[String],
    ) -> Option<ZodSchema> {
        let ty = op.signature.return_type.as_ref()?;
        (only_direct_return(op)
            && matches!(
                ty,
                hir::TypeSpec::SequenceType(_) | hir::TypeSpec::MapType(_)
            ))
        .then(|| zod_schema_for_type_spec_with_prefix(ty, module_path, Some("ifaceSchemas.models")))
    }
}
