use super::scope::SchemaScope;
use crate::error::IdlcError;
use crate::error::IdlcResult;
use crate::generate::openapi::naming::declarator_name;
use crate::generate::utils::{doc_lines_from_annotations, has_annotation};
use crate::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use crate::openapi::request_body::RequestBody;
use crate::openapi::schema::{
    AllOf, ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use crate::openapi::{Content, Deprecated, Ref, RefOr, Required};
use xidl_parser::hir;
use xidl_parser::rest_hir::semantics::{DeprecatedInfo, deprecated_info};

pub(crate) fn request_body_schema(schema: RefOr<Schema>, content_type: &str) -> RequestBody {
    let mut request_body = RequestBody::new();
    request_body
        .content
        .insert(content_type.to_string(), Content::new(Some(schema)));
    request_body
}

pub(crate) fn array_schema(items: RefOr<Schema>) -> RefOr<Schema> {
    RefOr::T(Schema::from(ArrayBuilder::new().items(items)))
}

pub(crate) fn parameter_schema(
    location: ParameterIn,
    name: &str,
    schema: RefOr<Schema>,
    required: bool,
    description: Option<String>,
) -> Parameter {
    let mut builder = ParameterBuilder::new()
        .name(name)
        .parameter_in(location)
        .required(if required {
            Required::True
        } else {
            Required::False
        })
        .schema(Some(schema));
    if let Some(description) = description {
        builder = builder.description(Some(description));
    }
    builder.build()
}

#[cfg(test)]
pub(crate) fn schema_for_struct(members: &[hir::Member]) -> RefOr<Schema> {
    let names = super::scope::SchemaNames::collect(&hir::Specification(Vec::new()));
    let units = super::units::DocumentUnits::default();
    SchemaScope::new(&names, &[], &units)
        .schema_for_struct_with_annotations(members, &[])
        .expect("schema")
}

pub(crate) fn apply_schema_description(
    mut schema: RefOr<Schema>,
    doc: Option<&str>,
) -> RefOr<Schema> {
    let Some(doc) = doc.filter(|value| !value.is_empty()) else {
        return schema;
    };
    match &mut schema {
        RefOr::T(Schema::Object(object)) => object.description = Some(doc.to_string()),
        RefOr::T(Schema::Array(array)) => array.description = Some(doc.to_string()),
        RefOr::T(Schema::OneOf(one_of)) => one_of.description = Some(doc.to_string()),
        RefOr::T(Schema::AllOf(all_of)) => all_of.description = Some(doc.to_string()),
        _ => {}
    }
    schema
}

pub(crate) fn doc_text(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    (!lines.is_empty()).then_some(lines.join("\n"))
}

pub(crate) fn apply_deprecation_note(
    description: Option<String>,
    deprecated: Option<&DeprecatedInfo>,
) -> Option<String> {
    if description.is_some() {
        return description;
    }
    deprecated.map(DeprecatedInfo::note)
}

/// The `@deprecated` declared on a schema-bearing declaration, as an
/// OpenAPI-facing error when its arguments are malformed.
pub(crate) fn schema_deprecation(
    annotations: &[hir::Annotation],
) -> IdlcResult<Option<DeprecatedInfo>> {
    deprecated_info(annotations).map_err(|reason| IdlcError::rpc(format!("@deprecated: {reason}")))
}

/// Flags an inline schema `deprecated: true`, adding the note as its
/// description when the declaration carries no documentation. `$ref`
/// schemas stay untouched: their target component carries its own flag.
pub(crate) fn apply_deprecation(
    schema: RefOr<Schema>,
    deprecated: Option<&DeprecatedInfo>,
) -> RefOr<Schema> {
    let Some(info) = deprecated else {
        return schema;
    };
    let note = info.note();
    match schema {
        RefOr::T(mut schema) => {
            match &mut schema {
                Schema::Object(object) => {
                    object.deprecated = Some(Deprecated::True);
                    object.description.get_or_insert(note);
                }
                Schema::Array(array) => {
                    array.deprecated = Some(Deprecated::True);
                    array.description.get_or_insert(note);
                }
                Schema::OneOf(one_of) => {
                    one_of.deprecated = Some(Deprecated::True);
                    one_of.description.get_or_insert(note);
                }
                Schema::AllOf(all_of) => {
                    all_of.deprecated = Some(Deprecated::True);
                    all_of.description.get_or_insert(note);
                }
                _ => {}
            }
            RefOr::T(schema)
        }
        RefOr::Ref(reference) => {
            // OpenAPI keeps annotation keywords beside `$ref`, so the flag
            // travels in a single-arm `allOf` wrapper.
            let mut combined = AllOf::new();
            combined.items.push(RefOr::Ref(reference));
            combined.deprecated = Some(Deprecated::True);
            combined.description.get_or_insert(note);
            RefOr::T(Schema::from(combined))
        }
    }
}

fn integer_schema(value: &hir::IntegerType) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Integer);
    match value {
        hir::IntegerType::U64 => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                .minimum(Some(0));
        }
        hir::IntegerType::U32 => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                .minimum(Some(0))
                .maximum(Some(u32::MAX));
        }
        hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::Octet
        | hir::IntegerType::UChar => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .minimum(Some(0));
        }
        hir::IntegerType::I64 => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)));
        }
        _ => object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32))),
    }
    RefOr::T(Schema::from(object))
}

pub(crate) fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
}

pub(crate) fn schema_ref(name: &str) -> RefOr<Schema> {
    RefOr::Ref(Ref::from_schema_name(name))
}

impl SchemaScope<'_> {
    /// Schema a scoped reference names, in this document or in another one.
    pub(crate) fn schema_reference(self, value: &hir::ScopedName) -> IdlcResult<RefOr<Schema>> {
        match self.units.reference(value) {
            Some(location) => Ok(RefOr::Ref(Ref::new(location))),
            None => Ok(schema_ref(&self.resolve(value)?)),
        }
    }

    pub(crate) fn schema_for_struct_with_annotations(
        self,
        members: &[hir::Member],
        container_annotations: &[hir::Annotation],
    ) -> IdlcResult<RefOr<Schema>> {
        let mut object = ObjectBuilder::new().schema_type(Type::Object);
        let mut flattened = Vec::new();
        for member in members {
            if hir::is_skipped(&member.annotations) {
                continue;
            }
            let optional = member.is_optional();
            let doc = doc_text(&member.annotations);
            let deprecated = schema_deprecation(&member.annotations)?;
            for decl in &member.ident {
                let name = hir::effective_wire_name(
                    &declarator_name(decl),
                    &member.annotations,
                    container_annotations,
                );
                let schema = apply_deprecation(
                    apply_schema_description(
                        self.schema_for_decl(&member.ty, decl)?,
                        doc.as_deref(),
                    ),
                    deprecated.as_ref(),
                );
                if has_annotation(&member.annotations, "flatten") {
                    flattened.push(schema);
                    continue;
                }
                object = object.property(name.clone(), schema);
                if !optional {
                    object = object.required(name);
                }
            }
        }
        let object = RefOr::T(Schema::from(object));
        Ok(if flattened.is_empty() {
            object
        } else {
            let mut combined = AllOf::new();
            combined.items.push(object);
            combined.items.extend(flattened);
            RefOr::T(Schema::from(combined))
        })
    }

    /// Combines a struct's own object schema with its base struct references.
    ///
    /// Struct inheritance is flattened on the wire, so the resulting schema is
    /// the intersection of the base schema and the struct's own fields.
    pub(crate) fn with_base_schemas(
        self,
        object: RefOr<Schema>,
        parents: &[hir::ScopedName],
    ) -> IdlcResult<RefOr<Schema>> {
        if parents.is_empty() {
            return Ok(object);
        }
        let mut combined = AllOf::new();
        combined.items.push(object);
        for parent in parents {
            combined.items.push(self.schema_reference(parent)?);
        }
        Ok(RefOr::T(Schema::from(combined)))
    }

    pub(crate) fn schema_for_union(self, def: &hir::UnionDef) -> IdlcResult<RefOr<Schema>> {
        let mut variants = Vec::new();
        for case in &def.case {
            let decl = &case.element.value;
            let name = declarator_name(decl);
            let schema = apply_schema_description(
                self.schema_for_element(&case.element.ty, decl)?,
                doc_text(&case.element.annotations).as_deref(),
            );
            let object = ObjectBuilder::new()
                .schema_type(Type::Object)
                .property(name.clone(), schema)
                .required(name);
            variants.push(RefOr::T(Schema::from(object)));
        }
        let mut one_of = OneOf::new();
        one_of.items = variants;
        Ok(RefOr::T(Schema::from(one_of)))
    }

    fn schema_for_element(
        self,
        ty: &hir::ElementSpecTy,
        decl: &hir::Declarator,
    ) -> IdlcResult<RefOr<Schema>> {
        match ty {
            hir::ElementSpecTy::TypeSpec(spec) => self.schema_for_decl(spec, decl),
            hir::ElementSpecTy::ConstrTypeDcl(constr) => Ok(self.schema_for_constr_type(constr)),
        }
    }

    fn schema_for_decl(
        self,
        ty: &hir::TypeSpec,
        decl: &hir::Declarator,
    ) -> IdlcResult<RefOr<Schema>> {
        Ok(Self::apply_declarator(self.schema_for_type(ty)?, decl))
    }

    /// Applies fixed-array dimensions with the first declaration dimension outermost.
    pub(super) fn apply_declarator(
        mut schema: RefOr<Schema>,
        decl: &hir::Declarator,
    ) -> RefOr<Schema> {
        if let hir::Declarator::ArrayDeclarator(array) = decl {
            for len in array.len.iter().rev() {
                let mut array_schema = ArrayBuilder::new().items(schema);
                if let Some(size) =
                    xidl_parser::hir::const_expr_to_i64(&len.0).filter(|size| *size >= 0)
                {
                    let size = size as usize;
                    array_schema = array_schema.min_items(Some(size)).max_items(Some(size));
                }
                schema = RefOr::T(Schema::from(array_schema));
            }
        }
        schema
    }

    pub(crate) fn schema_for_type(self, ty: &hir::TypeSpec) -> IdlcResult<RefOr<Schema>> {
        Ok(match ty {
            hir::TypeSpec::IntegerType(value) => integer_schema(value),
            hir::TypeSpec::FloatingPtType | hir::TypeSpec::FixedPtType(_) => {
                RefOr::T(Schema::from(
                    ObjectBuilder::new()
                        .schema_type(Type::Number)
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
                ))
            }
            hir::TypeSpec::CharType
            | hir::TypeSpec::WideCharType
            | hir::TypeSpec::StringType(_)
            | hir::TypeSpec::WideStringType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
            }
            hir::TypeSpec::Boolean => RefOr::T(Schema::from(
                ObjectBuilder::new().schema_type(Type::Boolean),
            )),
            hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
                RefOr::T(Schema::from(ObjectBuilder::new()))
            }
            hir::TypeSpec::ScopedName(value) => self.schema_reference(value)?,
            hir::TypeSpec::SequenceType(seq) => {
                let mut schema = ArrayBuilder::new().items(self.schema_for_type(&seq.ty)?);
                if let Some(size) = seq
                    .len
                    .as_ref()
                    .and_then(|len| xidl_parser::hir::const_expr_to_i64(&len.0))
                    .filter(|size| *size >= 0)
                {
                    let size = size as usize;
                    schema = schema.min_items(Some(size)).max_items(Some(size));
                }
                RefOr::T(Schema::from(schema))
            }
            hir::TypeSpec::MapType(map) => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .additional_properties(Some(self.schema_for_type(&map.value)?)),
            )),
            hir::TypeSpec::TemplateType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Object)))
            }
        })
    }

    pub(crate) fn schema_for_constr_type(self, constr: &hir::ConstrTypeDcl) -> RefOr<Schema> {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::StructForwardDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
            hir::ConstrTypeDcl::UnionForwardDcl(def) => {
                schema_ref(&super::naming::scoped_name(self.module_path, &def.ident))
            }
        }
    }
}
