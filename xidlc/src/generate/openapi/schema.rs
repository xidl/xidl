use xidl_parser::http_hir::semantics::DeprecatedInfo;
use crate::generate::openapi::naming::{declarator_name, field_rename};
use crate::generate::utils::doc_lines_from_annotations;
use crate::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use crate::openapi::request_body::RequestBody;
use crate::openapi::schema::{
    ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use crate::openapi::{Content, Ref, RefOr, Required};
use xidl_parser::hir;

pub(crate) fn body_payload_schema(
    props: Vec<(String, RefOr<Schema>)>,
    required: Vec<String>,
) -> Option<RefOr<Schema>> {
    if props.is_empty() {
        return None;
    }
    if props.len() == 1 {
        return props.into_iter().next().map(|(_, schema)| schema);
    }
    let mut object = ObjectBuilder::new().schema_type(Type::Object);
    for (name, schema) in props {
        object = object.property(name.clone(), schema);
    }
    for name in required {
        object = object.required(name);
    }
    Some(RefOr::T(Schema::from(object)))
}

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

pub(crate) fn schema_for_struct(members: &[hir::Member]) -> RefOr<Schema> {
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

pub(crate) fn schema_for_union(def: &hir::UnionDef) -> RefOr<Schema> {
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

fn schema_for_element(ty: &hir::ElementSpecTy, decl: &hir::Declarator) -> RefOr<Schema> {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => schema_for_decl(spec, decl),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => schema_for_constr_type(constr, &[]),
    }
}

fn schema_for_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> RefOr<Schema> {
    let mut schema = schema_for_type(ty);
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
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
    let Some(info) = deprecated else {
        return description;
    };
    let mut note = String::from("Deprecated.");
    match (&info.since, &info.after) {
        (Some(since), Some(after)) => note.push_str(&format!(" Since {since}. After {after}.")),
        (Some(since), None) => note.push_str(&format!(" Since {since}.")),
        (None, Some(after)) => note.push_str(&format!(" After {after}.")),
        (None, None) => {}
    }
    Some(note)
}

pub(crate) fn schema_for_type(ty: &hir::TypeSpec) -> RefOr<Schema> {
    match ty {
        hir::TypeSpec::IntegerType(value) => integer_schema(value),
        hir::TypeSpec::FloatingPtType | hir::TypeSpec::FixedPtType(_) => RefOr::T(Schema::from(
            ObjectBuilder::new()
                .schema_type(Type::Number)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
        )),
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
        hir::TypeSpec::ScopedName(value) => schema_ref(&scoped_name_ref(value)),
        hir::TypeSpec::SequenceType(seq) => {
            let mut schema = ArrayBuilder::new().items(schema_for_type(&seq.ty));
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
                .additional_properties(Some(schema_for_type(&map.value))),
        )),
        hir::TypeSpec::TemplateType(_) => {
            RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Object)))
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
        _ => object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32))),
    }
    RefOr::T(Schema::from(object))
}

pub(crate) fn schema_for_constr_type(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
) -> RefOr<Schema> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::EnumDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::UnionDef(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::BitsetDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::StructForwardDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::UnionForwardDcl(def) => {
            schema_ref(&super::naming::scoped_name(module_path, &def.ident))
        }
    }
}

pub(crate) fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
}

fn schema_ref(name: &str) -> RefOr<Schema> {
    RefOr::Ref(Ref::from_schema_name(name))
}

fn scoped_name_ref(value: &hir::ScopedName) -> String {
    value.name.join(".")
}
