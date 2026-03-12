use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

use crate::generate::utils::doc_lines_from_annotations;
use crate::jsonrpc::{Artifact, ArtifactFile};

pub(crate) struct OpenRpcCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenRpcCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::new())
    }

    async fn generate(
        &self,
        hir: Specification,
        _path: String,
        _props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let openrpc = render_openrpc(&hir);
        let content = serde_json::to_string_pretty(&openrpc)?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openrpc.json".to_string(),
            content,
        })])
    }
}

pub fn render_openrpc(spec: &hir::Specification) -> Value {
    let mut ctx = OpenRpcContext::default();
    ctx.collect_spec(spec, &[]);
    ctx.methods.sort_by(|left, right| {
        let left_name = left.get("name").and_then(Value::as_str).unwrap_or_default();
        let right_name = right
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        left_name.cmp(right_name)
    });

    let mut out = json!({
        "openrpc": "1.3.2",
        "info": {
            "title": ctx.info_title.as_deref().unwrap_or("xidl"),
            "version": ctx.info_version.as_deref().unwrap_or("0.1.0"),
        },
        "methods": ctx.methods,
    });

    if !ctx.schemas.is_empty() {
        out["components"] = json!({
            "schemas": ctx.schemas,
        });
    }

    out
}

#[derive(Default)]
struct OpenRpcContext {
    schemas: BTreeMap<String, Value>,
    methods: Vec<Value>,
    info_title: Option<String>,
    info_version: Option<String>,
}

impl OpenRpcContext {
    fn apply_pragma(&mut self, pragma: &hir::Pragma) {
        match pragma {
            hir::Pragma::XidlcPackage(value) => {
                if !value.is_empty() {
                    self.info_title = Some(value.clone());
                }
            }
            hir::Pragma::XidlcOpenApiVersion(value) => {
                if !value.is_empty() {
                    self.info_version = Some(value.clone());
                }
            }
            _ => {}
        }
    }

    fn collect_spec(&mut self, spec: &hir::Specification, module_path: &[String]) {
        for def in &spec.0 {
            self.collect_def(def, module_path);
        }
    }

    fn collect_def(&mut self, def: &hir::Definition, module_path: &[String]) {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                for nested in &module.definition {
                    self.collect_def(nested, &next_path);
                }
            }
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, module_path),
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::Definition::ExceptDcl(except) => {
                let name = scoped_name(module_path, &except.ident);
                self.schemas.insert(name, schema_for_struct(&except.member));
            }
            hir::Definition::InterfaceDcl(interface) => {
                self.collect_interface(interface, module_path)
            }
            hir::Definition::Pragma(pragma) => self.apply_pragma(pragma),
            _ => {}
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match &type_dcl.decl {
            hir::TypeDclInner::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(module_path, &declarator_name(decl));
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => schema_for_decl(ty, decl),
                        hir::TypedefType::ConstrTypeDcl(constr) => {
                            self.collect_constr_type(constr, module_path);
                            schema_for_constr_type(constr, module_path)
                        }
                    };
                    self.schemas.insert(name, schema);
                }
            }
            hir::TypeDclInner::ConstrTypeDcl(constr) => {
                self.collect_constr_type(constr, module_path)
            }
            hir::TypeDclInner::NativeDcl(_) => {}
        }
    }

    fn collect_constr_type(&mut self, constr: &hir::ConstrTypeDcl, module_path: &[String]) {
        let (name, schema) = match constr {
            hir::ConstrTypeDcl::StructDcl(def) => (
                scoped_name(module_path, &def.ident),
                apply_schema_description(
                    schema_for_struct(&def.member),
                    doc_text(&def.annotations),
                ),
            ),
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let values = def
                    .member
                    .iter()
                    .map(|value| Value::String(value.ident.clone()))
                    .collect::<Vec<_>>();
                (
                    scoped_name(module_path, &def.ident),
                    apply_schema_description(
                        json!({ "type": "string", "enum": values }),
                        doc_text(&def.annotations),
                    ),
                )
            }
            hir::ConstrTypeDcl::UnionDef(def) => (
                scoped_name(module_path, &def.ident),
                apply_schema_description(schema_for_union(def), doc_text(&def.annotations)),
            ),
            hir::ConstrTypeDcl::BitsetDcl(def) => (
                scoped_name(module_path, &def.ident),
                apply_schema_description(json!({ "type": "integer" }), doc_text(&def.annotations)),
            ),
            hir::ConstrTypeDcl::BitmaskDcl(def) => (
                scoped_name(module_path, &def.ident),
                apply_schema_description(json!({ "type": "integer" }), doc_text(&def.annotations)),
            ),
            hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
                return;
            }
        };
        self.schemas.insert(name, schema);
    }

    fn collect_interface(&mut self, interface: &hir::InterfaceDcl, module_path: &[String]) {
        let def = match &interface.decl {
            hir::InterfaceDclInner::InterfaceDef(def) => def,
            _ => return,
        };

        let mut user_ops = HashSet::new();
        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                if let hir::Export::OpDcl(op) = export {
                    user_ops.insert(op.ident.clone());
                }
            }
        }

        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        self.methods
                            .push(render_op(op, &def.header.ident, module_path));
                    }
                    hir::Export::AttrDcl(attr) => {
                        self.methods.extend(render_attr(
                            attr,
                            &def.header.ident,
                            module_path,
                            &user_ops,
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum StreamKind {
    Server,
    Client,
    Bidi,
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> Value {
    let mut params = Vec::new();
    let mut outputs = Vec::new();
    let stream_kind = stream_kind_from_annotations(&op.annotations);

    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        outputs.push(("return".to_string(), schema_for_type(ty)));
    }

    let param_list = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    for param in param_list {
        let direction = param_direction(param.attr.as_ref());
        let name = param.declarator.0.clone();
        let schema = schema_for_type(&param.ty);

        if matches!(direction, ParamDirection::In | ParamDirection::InOut) {
            let mut param_value = json!({
                "name": name,
                "required": !has_optional_annotation(&param.annotations),
                "schema": schema,
            });
            if let Some(description) = doc_text(&param.annotations) {
                param_value["description"] = Value::String(description);
            }
            params.push(param_value);
        }
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            outputs.push((name, schema));
        }
    }

    let mut method = json!({
        "name": rpc_method_name(module_path, interface_name, &op.ident),
        "params": params,
        "result": {
            "name": "result",
            "schema": result_object_schema(outputs),
        },
    });
    if let Some(description) = doc_text(&op.annotations) {
        method["description"] = Value::String(description);
    }

    if let Some(kind) = stream_kind {
        method["x-xidl-stream"] = stream_extension(kind, module_path, interface_name, &op.ident);
    }

    method
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
    user_ops: &HashSet<String>,
) -> Vec<Value> {
    let emit_watch = has_annotation(&attr.annotations, "server_stream");
    let attr_doc = doc_text(&attr.annotations);
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|name| {
                let getter = format!("get_attribute_{name}");
                validate_attr_collision(user_ops, &name, &getter, None);
                let mut method = json!({
                    "name": rpc_method_name(module_path, interface_name, &getter),
                    "params": [],
                    "result": {
                        "name": "result",
                        "schema": result_object_schema(vec![("return".to_string(), schema_for_type(&spec.ty))]),
                    }
                });
                if let Some(description) = attr_doc.clone() {
                    method["description"] = Value::String(description);
                }
                if emit_watch {
                    method["x-xidl-stream"] = stream_extension_direct(StreamKind::Server);
                }
                method
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let name = decl.0.clone();
                        let getter = format!("get_attribute_{name}");
                        let setter = format!("set_attribute_{name}");
                        validate_attr_collision(user_ops, &name, &getter, Some(&setter));

                        let mut getter_method = json!({
                            "name": rpc_method_name(module_path, interface_name, &getter),
                            "params": [],
                            "result": {
                                "name": "result",
                                "schema": result_object_schema(vec![("return".to_string(), schema_for_type(&spec.ty))]),
                            }
                        });
                        if let Some(description) = attr_doc.clone() {
                            getter_method["description"] = Value::String(description);
                        }
                        if emit_watch {
                            getter_method["x-xidl-stream"] =
                                stream_extension_direct(StreamKind::Server);
                        }
                        out.push(getter_method);

                        let mut setter_method = json!({
                            "name": rpc_method_name(module_path, interface_name, &setter),
                            "params": [{
                                "name": name,
                                "required": true,
                                "schema": schema_for_type(&spec.ty),
                            }],
                            "result": {
                                "name": "result",
                                "schema": result_object_schema(Vec::new()),
                            }
                        });
                        if let Some(description) = attr_doc.clone() {
                            setter_method["params"][0]["description"] = Value::String(description);
                        }
                        out.push(setter_method);
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let name = declarator.0.clone();
                    let getter = format!("get_attribute_{name}");
                    let setter = format!("set_attribute_{name}");
                    validate_attr_collision(user_ops, &name, &getter, Some(&setter));

                    let mut getter_method = json!({
                        "name": rpc_method_name(module_path, interface_name, &getter),
                        "params": [],
                        "result": {
                            "name": "result",
                            "schema": result_object_schema(vec![("return".to_string(), schema_for_type(&spec.ty))]),
                        }
                    });
                    if let Some(description) = attr_doc.clone() {
                        getter_method["description"] = Value::String(description);
                    }
                    if emit_watch {
                        getter_method["x-xidl-stream"] =
                            stream_extension_direct(StreamKind::Server);
                    }
                    out.push(getter_method);

                    let mut setter_method = json!({
                        "name": rpc_method_name(module_path, interface_name, &setter),
                        "params": [{
                            "name": name,
                            "required": true,
                            "schema": schema_for_type(&spec.ty),
                        }],
                        "result": {
                            "name": "result",
                            "schema": result_object_schema(Vec::new()),
                        }
                    });
                    if let Some(description) = attr_doc.clone() {
                        setter_method["params"][0]["description"] = Value::String(description);
                    }
                    out.push(setter_method);
                }
            }
            out
        }
    }
}

fn validate_attr_collision(
    user_ops: &HashSet<String>,
    attr_name: &str,
    getter: &str,
    setter: Option<&str>,
) {
    let getter_conflict = user_ops.contains(getter);
    let setter_conflict = setter.map(|name| user_ops.contains(name)).unwrap_or(false);
    if getter_conflict || setter_conflict {
        let setter_text = setter
            .map(|value| format!(" or `{value}`"))
            .unwrap_or_default();
        panic!(
            "attribute `{attr_name}` conflicts with user-defined operation `{getter}`{setter_text}"
        );
    }
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn result_object_schema(fields: Vec<(String, Value)>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for (name, schema) in fields {
        properties.insert(name.clone(), schema);
        required.push(Value::String(name));
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

fn schema_for_struct(members: &[hir::Member]) -> Value {
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

fn schema_for_union(def: &hir::UnionDef) -> Value {
    let variants = def
        .case
        .iter()
        .filter(|case| !is_internal_rpc_marker_element(&case.element.ty))
        .map(|case| {
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
        })
        .collect::<Vec<_>>();

    if variants.is_empty() {
        json!({ "type": "object" })
    } else {
        json!({ "oneOf": variants })
    }
}

fn schema_for_element(ty: &hir::ElementSpecTy, decl: &hir::Declarator) -> Value {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => schema_for_decl(spec, decl),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => schema_for_constr_type(constr, &[]),
    }
}

fn schema_for_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> Value {
    let mut schema = schema_for_type(ty);
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            let mut array_schema = json!({
                "type": "array",
                "items": schema,
            });
            if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                if size >= 0 {
                    array_schema["minItems"] = Value::Number((size as u64).into());
                    array_schema["maxItems"] = Value::Number((size as u64).into());
                }
            }
            schema = array_schema;
        }
    }
    schema
}

fn schema_for_constr_type(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> Value {
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

fn schema_for_type(ty: &hir::TypeSpec) -> Value {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_schema(value),
            hir::SimpleTypeSpec::FloatingPtType => json!({ "type": "number", "format": "double" }),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                json!({ "type": "string" })
            }
            hir::SimpleTypeSpec::Boolean => json!({ "type": "boolean" }),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => json!({}),
            hir::SimpleTypeSpec::ScopedName(value) => {
                let scoped = value.name.join(".");
                match scoped.as_str() {
                    "dds.rpc.UnusedMember" => {
                        json!({ "type": "object", "properties": {}, "required": [] })
                    }
                    "dds.rpc.RequestHeader" | "dds.rpc.ReplyHeader" => {
                        json!({ "type": "object", "additionalProperties": true })
                    }
                    "dds.rpc.UnknownOperation" => json!({ "type": "object" }),
                    _ => schema_ref(&scoped),
                }
            }
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let mut out = json!({
                    "type": "array",
                    "items": schema_for_type(&seq.ty),
                });
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if size >= 0 {
                            out["minItems"] = Value::Number((size as u64).into());
                            out["maxItems"] = Value::Number((size as u64).into());
                        }
                    }
                }
                out
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                json!({ "type": "string" })
            }
            hir::TemplateTypeSpec::FixedPtType(_) => {
                json!({ "type": "number", "format": "double" })
            }
            hir::TemplateTypeSpec::MapType(map) => json!({
                "type": "object",
                "additionalProperties": schema_for_type(&map.value),
            }),
            hir::TemplateTypeSpec::TemplateType(_) => json!({ "type": "object" }),
        },
    }
}

fn integer_schema(value: &hir::IntegerType) -> Value {
    match value {
        hir::IntegerType::U64 => json!({ "type": "integer", "format": "int64", "minimum": 0 }),
        hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar => {
            json!({ "type": "integer", "format": "int32", "minimum": 0 })
        }
        hir::IntegerType::I64 => json!({ "type": "integer", "format": "int64" }),
        _ => json!({ "type": "integer", "format": "int32" }),
    }
}

fn schema_ref(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

fn apply_schema_description(mut schema: Value, doc: Option<String>) -> Value {
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

fn doc_text(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xidl_parser::hir;

    fn doc_annotation(text: &str) -> hir::Annotation {
        hir::Annotation::Builtin {
            name: "doc".to_string(),
            params: Some(hir::AnnotationParams::Raw(format!("\"{}\"", text))),
        }
    }

    #[test]
    fn schema_for_struct_applies_doc_to_fields() {
        let member = hir::Member {
            annotations: vec![doc_annotation("field doc")],
            ty: hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::IntegerType(
                hir::IntegerType::I32,
            )),
            ident: vec![hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
                "value".to_string(),
            ))],
            default: None,
            field_id: None,
        };
        let schema = schema_for_struct(&[member]);
        let Value::Object(map) = schema else {
            panic!("expected object schema");
        };
        let Value::Object(props) = map.get("properties").expect("properties") else {
            panic!("expected properties");
        };
        let Value::Object(value_schema) = props.get("value").expect("value") else {
            panic!("expected value schema");
        };
        assert_eq!(
            value_schema.get("description").and_then(Value::as_str),
            Some("field doc")
        );
    }
}

fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        let mut parts = module_path.to_vec();
        parts.push(ident.to_string());
        parts.join(".")
    }
}

fn rpc_method_name(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("optional"))
            .unwrap_or(false)
    })
}

fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

fn stream_kind_from_annotations(annotations: &[hir::Annotation]) -> Option<StreamKind> {
    let mut out = None;
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("server_stream") {
            Some(StreamKind::Server)
        } else if name.eq_ignore_ascii_case("client_stream") {
            Some(StreamKind::Client)
        } else if name.eq_ignore_ascii_case("bidi_stream") {
            Some(StreamKind::Bidi)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        match out {
            None => out = Some(current),
            Some(prev) if prev == current => {}
            Some(_) => panic!("@server_stream/@client_stream/@bidi_stream are mutually exclusive"),
        }
    }
    out
}

fn stream_mode_name(kind: StreamKind) -> &'static str {
    match kind {
        StreamKind::Server => "server",
        StreamKind::Client => "client",
        StreamKind::Bidi => "bidi",
    }
}

fn stream_extension(
    kind: StreamKind,
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> Value {
    let _ = (module_path, interface_name, method_name);
    stream_extension_direct(kind)
}

fn stream_extension_direct(kind: StreamKind) -> Value {
    json!({
        "mode": stream_mode_name(kind),
        "codec": "json",
        "delivery": "direct",
    })
}

fn is_internal_rpc_marker_type(ty: &hir::TypeSpec) -> bool {
    matches!(
        ty,
        hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value))
            if matches!(value.name.join(".").as_str(), "dds.rpc.UnusedMember" | "dds.rpc.RequestHeader" | "dds.rpc.ReplyHeader")
    )
}

fn is_internal_rpc_marker_element(ty: &hir::ElementSpecTy) -> bool {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => {
            matches!(
                spec,
                hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value))
                    if value.name.join(".") == "dds.rpc.UnknownOperation"
            )
        }
        hir::ElementSpecTy::ConstrTypeDcl(_) => false,
    }
}
