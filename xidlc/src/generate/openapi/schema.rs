use crate::generate::openapi::utils::{declarator_name, doc_text, field_rename, scoped_name};
use crate::openapi::schema::{
    ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use crate::openapi::{Ref, RefOr};
use xidl_parser::hir;

/// Returns an OpenAPI schema for the given HIR type.
pub fn schema_for_type(ty: &hir::TypeSpec) -> RefOr<Schema> {
    match ty {
        hir::TypeSpec::IntegerType(value) => integer_schema(value),
        hir::TypeSpec::FloatingPtType => RefOr::T(Schema::from(
            ObjectBuilder::new()
                .schema_type(Type::Number)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
        )),
        hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => {
            RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
        }
        hir::TypeSpec::Boolean => RefOr::T(Schema::from(
            ObjectBuilder::new().schema_type(Type::Boolean),
        )),
        hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
            RefOr::T(Schema::from(ObjectBuilder::new()))
        }
        hir::TypeSpec::ScopedName(value) => schema_ref(&scoped_name_ref(value)),
        hir::TypeSpec::SequenceType(seq) => {
            let mut schema = ArrayBuilder::new().items(schema_for_type(&seq.ty));
            if let Some(len) = &seq.len
                && let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0)
                && size >= 0
            {
                let size = size as usize;
                schema = schema.min_items(Some(size)).max_items(Some(size));
            }
            RefOr::T(Schema::from(schema))
        }
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => {
            RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
        }
        hir::TypeSpec::FixedPtType(_) => RefOr::T(Schema::from(
            ObjectBuilder::new()
                .schema_type(Type::Number)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
        )),
        hir::TypeSpec::MapType(map) => RefOr::T(Schema::from(
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .additional_properties(Some(schema_for_type(&map.value))),
        )),
        hir::TypeSpec::TemplateType(_) => {
            RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Object)))
        }
    }
}

/// Returns an OpenAPI schema for the given HIR integer type.
pub fn integer_schema(value: &hir::IntegerType) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Integer);
    match value {
        hir::IntegerType::U64 => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                .minimum(Some(0));
        }
        hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .minimum(Some(0));
        }
        hir::IntegerType::I64 => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)));
        }
        _ => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)));
        }
    }
    RefOr::T(Schema::from(object))
}

/// Returns an OpenAPI schema for the given HIR struct members.
pub fn schema_for_struct(members: &[hir::Member]) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Object);
    for member in members {
        let rename = field_rename(&member.annotations);
        let optional = member.is_optional();
        let doc = doc_text(&member.annotations);
        for decl in &member.ident {
            let name = rename.clone().unwrap_or_else(|| declarator_name(decl));
            let schema =
                apply_schema_description(schema_for_decl(&member.ty, decl), doc.as_deref());
            object = object.property(name.clone(), schema);
            if !optional {
                object = object.required(name);
            }
        }
    }
    RefOr::T(Schema::from(object))
}

/// Returns an OpenAPI schema for the given HIR union definition.
pub fn schema_for_union(def: &hir::UnionDef) -> RefOr<Schema> {
    let mut variants = Vec::new();
    for case in &def.case {
        let decl = &case.element.value;
        let name = declarator_name(decl);
        let schema = apply_schema_description(
            schema_for_element(&case.element.ty, decl),
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
    RefOr::T(Schema::from(one_of))
}

/// Returns an OpenAPI schema for the given HIR element.
pub fn schema_for_element(ty: &hir::ElementSpecTy, decl: &hir::Declarator) -> RefOr<Schema> {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => schema_for_decl(spec, decl),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => schema_for_constr_type(constr, &[]),
    }
}

/// Returns an OpenAPI schema for the given HIR declarator.
pub fn schema_for_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> RefOr<Schema> {
    let mut schema = schema_for_type(ty);
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            let size = xidl_parser::hir::const_expr_to_i64(&len.0);
            let mut array_schema = ArrayBuilder::new().items(schema);
            if let Some(size) = size {
                if size >= 0 {
                    let size = size as usize;
                    array_schema = array_schema.min_items(Some(size)).max_items(Some(size));
                }
            }
            schema = RefOr::T(Schema::from(array_schema));
        }
    }
    schema
}

/// Returns an OpenAPI schema for the given HIR constructed type.
pub fn schema_for_constr_type(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
) -> RefOr<Schema> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::EnumDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::UnionDef(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::BitsetDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::StructForwardDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::UnionForwardDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
    }
}

/// Applies the given description to the OpenAPI schema.
pub fn apply_schema_description(mut schema: RefOr<Schema>, doc: Option<&str>) -> RefOr<Schema> {
    let Some(doc) = doc.filter(|value| !value.is_empty()) else {
        return schema;
    };
    match &mut schema {
        RefOr::T(Schema::Object(object)) => {
            object.description = Some(doc.to_string());
        }
        RefOr::T(Schema::Array(array)) => {
            array.description = Some(doc.to_string());
        }
        RefOr::T(Schema::OneOf(one_of)) => {
            one_of.description = Some(doc.to_string());
        }
        _ => {}
    }
    schema
}

/// Returns a reference to the schema with the given name.
pub fn schema_ref(name: &str) -> RefOr<Schema> {
    RefOr::Ref(Ref::from_schema_name(name))
}

/// Returns a reference to the error schema.
pub fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
}

/// Returns an OpenAPI array schema for the given item schema.
pub fn array_schema(items: RefOr<Schema>) -> RefOr<Schema> {
    RefOr::T(Schema::from(ArrayBuilder::new().items(items)))
}

/// Options for creating an OpenAPI parameter.
pub struct ParameterOptions {
    /// The location of the parameter.
    pub location: crate::openapi::path::ParameterIn,
    /// The name of the parameter.
    pub name: String,
    /// The schema of the parameter.
    pub schema: RefOr<Schema>,
    /// Whether the parameter is required.
    pub required: bool,
    /// An optional description of the parameter.
    pub description: Option<String>,
}

/// Returns an OpenAPI parameter schema.
pub fn parameter_schema(opts: ParameterOptions) -> crate::openapi::path::Parameter {
    let required = if opts.required {
        crate::openapi::Required::True
    } else {
        crate::openapi::Required::False
    };
    let mut builder = crate::openapi::path::ParameterBuilder::new()
        .name(&opts.name)
        .parameter_in(opts.location)
        .required(required)
        .schema(Some(opts.schema));
    if let Some(description) = opts.description {
        builder = builder.description(Some(description));
    }
    builder.build()
}

fn scoped_name_ref(value: &hir::ScopedName) -> String {
    value.name.join(".")
}
