use serde_json::{Map, Value, json};
use xidl_parser::hir;

use super::annotations::doc_text;
use super::names::{declarator_name, scoped_name};
use super::schema_types::{
    is_internal_rpc_marker_element, is_internal_rpc_marker_type, schema_for_type,
};

pub(super) fn schema_for_struct(members: &[hir::Member]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for member in members {
        if is_internal_rpc_marker_type(&member.ty) {
            continue;
        }
        let doc = doc_text(&member.annotations);
        for decl in &member.ident {
            let name = declarator_name(decl);
            let schema = apply_schema_description(schema_for_decl(&member.ty, decl), doc.clone());
            properties.insert(name.clone(), schema);
            if !member.is_optional() {
                required.push(Value::String(name));
            }
        }
    }

    let mut out = json!({
        "type": "object",
        "properties": properties,
    });
    if !required.is_empty() {
        out["required"] = Value::Array(required);
    }
    out
}

pub(super) fn schema_for_union(def: &hir::UnionDef) -> Value {
    let variants = def
        .case
        .iter()
        .filter(|case| !is_internal_rpc_marker_element(&case.element.ty))
        .map(union_variant_schema)
        .collect::<Vec<_>>();

    if variants.is_empty() {
        json!({ "type": "object" })
    } else {
        json!({ "oneOf": variants })
    }
}

fn union_variant_schema(case: &hir::Case) -> Value {
    let name = declarator_name(&case.element.value);
    let schema = apply_schema_description(
        schema_for_element(&case.element.ty, &case.element.value),
        doc_text(&case.element.annotations),
    );
    let mut properties = Map::new();
    properties.insert(name.clone(), schema);
    Value::Object(
        [
            ("type".to_string(), Value::String("object".to_string())),
            ("properties".to_string(), Value::Object(properties)),
            (
                "required".to_string(),
                Value::Array(vec![Value::String(name)]),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

fn schema_for_element(ty: &hir::ElementSpecTy, decl: &hir::Declarator) -> Value {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => schema_for_decl(spec, decl),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => schema_for_constr_type(constr, &[]),
    }
}

pub(super) fn schema_for_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> Value {
    let mut schema = schema_for_type(ty);
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            schema = array_with_bounds(schema, len);
        }
    }
    schema
}

fn array_with_bounds(items: Value, len: &hir::PositiveIntConst) -> Value {
    let mut array_schema = json!({
        "type": "array",
        "items": items,
    });
    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
        if size >= 0 {
            array_schema["minItems"] = Value::Number((size as u64).into());
            array_schema["maxItems"] = Value::Number((size as u64).into());
        }
    }
    array_schema
}

pub(super) fn schema_for_constr_type(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> Value {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => schema_ref(&scoped_name(module_path, &def.ident)),
        hir::ConstrTypeDcl::EnumDcl(def) => schema_ref(&scoped_name(module_path, &def.ident)),
        hir::ConstrTypeDcl::UnionDef(def) => schema_ref(&scoped_name(module_path, &def.ident)),
        hir::ConstrTypeDcl::BitsetDcl(def) => schema_ref(&scoped_name(module_path, &def.ident)),
        hir::ConstrTypeDcl::BitmaskDcl(def) => schema_ref(&scoped_name(module_path, &def.ident)),
        hir::ConstrTypeDcl::StructForwardDcl(def) => {
            schema_ref(&scoped_name(module_path, &def.ident))
        }
        hir::ConstrTypeDcl::UnionForwardDcl(def) => {
            schema_ref(&scoped_name(module_path, &def.ident))
        }
    }
}

fn schema_ref(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

pub(super) fn apply_schema_description(mut schema: Value, doc: Option<String>) -> Value {
    let Some(doc) = doc.filter(|value| !value.is_empty()) else {
        return schema;
    };
    if let Value::Object(ref map) = schema {
        if map.contains_key("$ref") {
            return schema;
        }
    }
    if let Value::Object(ref mut map) = schema {
        if !map.contains_key("description") {
            map.insert("description".to_string(), Value::String(doc));
        }
    }
    schema
}
