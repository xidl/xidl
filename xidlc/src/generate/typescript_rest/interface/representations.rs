use super::super::model::TsRepresentationContext;
use crate::generate::typescript::definition::TypeRefTarget;
use crate::generate::typescript::definition::contexts::{
    FieldTypeContext, ParamDeclContext, TsType,
};
use crate::generate::typescript::definition::type_expr::{
    ts_type_for_type_spec, zod_schema_for_type_spec_with_prefix,
};
use xidl_parser::rest_hir::HttpOperation;

impl TsRepresentationContext {
    pub(super) fn for_operation(op: &HttpOperation, module_path: &[String]) -> Vec<Self> {
        op.http
            .response
            .representations
            .iter()
            .map(|repr| TsRepresentationContext {
                kind: repr.content_type.idl_name().to_string(),
                content_type: repr.content_type.as_str().to_string(),
                value_ty: ts_type_for_type_spec(&repr.ty, module_path, TypeRefTarget::Client),
                schema: zod_schema_for_type_spec_with_prefix(
                    &repr.ty,
                    module_path,
                    Some("ifaceSchemas.models"),
                ),
                is_byte: repr.is_byte,
            })
            .collect()
    }
    pub(super) fn response_type(
        op: &HttpOperation,
        representations: &[Self],
        module_path: &[String],
        response_fields: &[ParamDeclContext],
    ) -> Option<TsType> {
        (!representations.is_empty()).then(|| {
            let metadata_fields: Vec<FieldTypeContext> =
                op.http
                    .response
                    .header
                    .iter()
                    .chain(&op.http.response.cookie)
                    .filter_map(|binding| match &binding.source {
                        xidl_parser::rest_hir::HttpOutputSource::Param { name } => {
                            Some(FieldTypeContext {
                                prop: name.clone(),
                                ty: ts_type_for_type_spec(
                                    &binding.ty,
                                    module_path,
                                    TypeRefTarget::Client,
                                ),
                                optional: response_fields.iter().any(|field| {
                                    field.prop
                            == crate::generate::typescript::definition::names::ts_prop_name(name)
                            && field.optional
                                }),
                                doc: Vec::new(),
                            })
                        }
                        xidl_parser::rest_hir::HttpOutputSource::ReturnValue => None,
                    })
                    .collect();
            TsType::Union(
                representations
                    .iter()
                    .map(|repr| {
                        let mut fields = vec![
                            FieldTypeContext {
                                prop: "kind".to_string(),
                                ty: TsType::Primitive(format!("\"{}\"", repr.kind)),
                                optional: false,
                                doc: Vec::new(),
                            },
                            FieldTypeContext {
                                prop: "value".to_string(),
                                ty: if repr.is_byte {
                                    TsType::Primitive("Uint8Array".to_string())
                                } else {
                                    repr.value_ty.clone()
                                },
                                optional: false,
                                doc: Vec::new(),
                            },
                        ];
                        fields.extend(metadata_fields.iter().cloned());
                        TsType::InlineStruct(fields)
                    })
                    .collect(),
            )
        })
    }
}
