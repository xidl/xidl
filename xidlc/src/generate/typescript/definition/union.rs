//! Type and schema projections of an IDL union's shared wire layout.

use super::contexts::{FieldTypeContext, FieldZodContext, TsType, ZodSchema};
use super::method::TypeRefTarget;
use super::names::{declarator_name, ts_prop_name};
use super::type_expr::{ts_type_for_element, zod_schema_for_element};
use crate::generate::utils::{has_annotation, union_wire::UnionWire};
use xidl_parser::hir;

pub(super) struct TsUnion<'a> {
    pub(super) definition: &'a hir::UnionDef,
    pub(super) module_path: &'a [String],
}

impl TsUnion<'_> {
    pub(super) fn types(&self, target: TypeRefTarget) -> Vec<TsType> {
        self.definition
            .case
            .iter()
            .map(|case| {
                let payload = ts_type_for_element(
                    &case.element.ty,
                    &case.element.value,
                    self.module_path,
                    target,
                );
                let fields = if has_annotation(&self.definition.annotations, "http") {
                    vec![Self::field(
                        ts_prop_name(declarator_name(&case.element.value)),
                        payload,
                    )]
                } else {
                    vec![
                        Self::field("tag".into(), TsType::InlineEnum(UnionWire::tags(case))),
                        Self::field("data".into(), payload),
                    ]
                };
                TsType::InlineStruct(fields)
            })
            .collect()
    }

    pub(super) fn schemas(&self) -> Vec<ZodSchema> {
        self.definition
            .case
            .iter()
            .map(|case| {
                let payload =
                    zod_schema_for_element(&case.element.ty, &case.element.value, self.module_path);
                let fields = if has_annotation(&self.definition.annotations, "http") {
                    vec![Self::schema_field(
                        ts_prop_name(declarator_name(&case.element.value)),
                        payload,
                    )]
                } else {
                    vec![
                        Self::schema_field("tag".into(), ZodSchema::Enum(UnionWire::tags(case))),
                        Self::schema_field("data".into(), payload),
                    ]
                };
                ZodSchema::Object(fields)
            })
            .collect()
    }

    fn field(prop: String, ty: TsType) -> FieldTypeContext {
        FieldTypeContext {
            prop,
            ty,
            optional: false,
            doc: Vec::new(),
        }
    }

    fn schema_field(prop: String, schema: ZodSchema) -> FieldZodContext {
        FieldZodContext {
            prop,
            schema,
            optional: false,
            xjson_meta: None,
        }
    }
}
