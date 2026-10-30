//! Union response schemas use the same discriminator facts as generated codecs.

use super::*;
use crate::generate::utils::union_wire::UnionWire;

impl SchemaScope<'_> {
    /// Projects case tags and payloads without introducing a target-specific layout.
    pub(crate) fn schema_for_union(self, definition: &hir::UnionDef) -> IdlcResult<RefOr<Schema>> {
        let mut variants = Vec::new();
        for case in &definition.case {
            let payload = apply_schema_description(
                self.schema_for_element(&case.element.ty, &case.element.value)?,
                doc_text(&case.element.annotations).as_deref(),
            );
            let object = if has_annotation(&definition.annotations, "http") {
                let name = declarator_name(&case.element.value);
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .property(name.clone(), payload)
                    .required(name)
            } else {
                let tag = ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(UnionWire::tags(case)));
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .property("tag", tag)
                    .required("tag")
                    .property("data", payload)
                    .required("data")
            };
            variants.push(RefOr::T(Schema::from(object)));
        }
        let mut one_of = OneOf::new();
        one_of.items = variants;
        Ok(RefOr::T(Schema::from(one_of)))
    }
}
