use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use xidl_parser::hir;

pub fn render_openapi(spec: &hir::Specification) -> Value {
    let mut ctx = OpenApiContext::default();
    ctx.collect_spec(spec, &[]);

    let mut schemas = Map::new();
    for (name, schema) in ctx.schemas {
        schemas.insert(name, schema);
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "xidl",
            "version": "0.1.0"
        },
        "paths": ctx.paths,
        "components": {
            "schemas": schemas,
        }
    })
}

#[derive(Default)]
struct OpenApiContext {
    schemas: BTreeMap<String, Value>,
    paths: Map<String, Value>,
}

impl OpenApiContext {
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
                for def in &module.definition {
                    self.collect_def(def, &next_path);
                }
            }
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, module_path),
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::Definition::ExceptDcl(except) => self.collect_exception(except, module_path),
            hir::Definition::InterfaceDcl(interface) => {
                self.collect_interface(interface, module_path)
            }
            _ => {}
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match &type_dcl.decl {
            hir::TypeDclInner::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(module_path, &declarator_name(decl));
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => schema_for_type(ty),
                        hir::TypedefType::ConstrTypeDcl(constr) => {
                            self.collect_constr_type(constr, module_path);
                            schema_for_constr_type(constr, module_path)
                        }
                    };
                    self.schemas.insert(name, schema);
                }
            }
            hir::TypeDclInner::ConstrTypeDcl(constr) => {
                self.collect_constr_type(constr, module_path);
            }
            hir::TypeDclInner::NativeDcl(_) => {}
        }
    }

    fn collect_constr_type(&mut self, constr: &hir::ConstrTypeDcl, module_path: &[String]) {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = schema_for_struct(&def.member);
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let values = def.member.iter().map(|v| Value::String(v.ident.clone()));
                let schema = json!({
                    "type": "string",
                    "enum": values.collect::<Vec<_>>()
                });
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = schema_for_union(def);
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                self.schemas.insert(name, json!({"type": "integer"}));
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                self.schemas.insert(name, json!({"type": "integer"}));
            }
            hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
        }
    }

    fn collect_exception(&mut self, except: &hir::ExceptDcl, module_path: &[String]) {
        let name = scoped_name(module_path, &except.ident);
        let schema = schema_for_struct(&except.member);
        self.schemas.insert(name, schema);
    }

    fn collect_interface(&mut self, interface: &hir::InterfaceDcl, module_path: &[String]) {
        let def = match &interface.decl {
            hir::InterfaceDclInner::InterfaceDef(def) => def,
            _ => return,
        };
        let mut methods = Vec::new();
        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        methods.push(render_op(op, &def.header.ident, module_path));
                    }
                    hir::Export::AttrDcl(attr) => {
                        methods.extend(render_attr(attr, &def.header.ident, module_path));
                    }
                    _ => {}
                }
            }
        }

        for method in methods {
            let entry = self
                .paths
                .entry(method.path.clone())
                .or_insert_with(|| json!({}));
            let mut operation = json!({
                "operationId": method.operation_id,
                "responses": {
                    "200": {
                        "description": "OK",
                        "content": {
                            "application/json": {
                                "schema": method.response_schema
                            }
                        }
                    },
                    "500": {
                        "description": "Error",
                        "content": {
                            "application/json": {
                                "schema": error_schema_ref()
                            }
                        }
                    }
                }
            });

            if !method.parameters.is_empty() {
                operation
                    .as_object_mut()
                    .unwrap()
                    .insert("parameters".to_string(), Value::Array(method.parameters));
            }
            if let Some(request_body) = method.request_body {
                operation
                    .as_object_mut()
                    .unwrap()
                    .insert("requestBody".to_string(), request_body);
            }

            let entry = entry.as_object_mut().unwrap();
            entry.insert(method.http_method, operation);
        }

        self.schemas.entry("Error".to_string()).or_insert_with(|| {
            json!({
                "type": "object",
                "properties": {
                    "code": { "type": "integer" },
                    "msg": { "type": "string" }
                },
                "required": ["code", "msg"]
            })
        });
    }
}

struct MethodInfo {
    http_method: String,
    path: String,
    operation_id: String,
    parameters: Vec<Value>,
    request_body: Option<Value>,
    response_schema: Value,
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodInfo {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => json!({ "type": "null" }),
        hir::OpTypeSpec::TypeSpec(ty) => schema_for_type(ty),
    };

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    let (method, path) = route_from_annotations(
        &op.annotations,
        HttpMethod::Post,
        default_path(module_path, interface_name, &op.ident),
    );
    let path_param_names = parse_path_params(&path);
    let default_source = default_param_source(method);

    let mut parameters = Vec::new();
    let mut body_props = Map::new();
    let mut body_required = Vec::new();

    for param in params {
        let raw_name = param.declarator.0.clone();
        let schema = schema_for_type(&param.ty);
        let source = if path_param_names.contains(&raw_name) {
            ParamSource::Path
        } else {
            default_source
        };
        match source {
            ParamSource::Path => parameters.push(parameter_schema("path", &raw_name, schema, true)),
            ParamSource::Query => {
                parameters.push(parameter_schema("query", &raw_name, schema, true))
            }
            ParamSource::Body => {
                body_props.insert(raw_name.clone(), schema);
                body_required.push(Value::String(raw_name));
            }
        }
    }

    MethodInfo {
        http_method: method_to_openapi(method),
        path,
        operation_id: operation_id(module_path, interface_name, &op.ident),
        parameters,
        request_body: body_schema(body_props, body_required),
        response_schema: ret,
    }
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_name: &str,
    module_path: &[String],
) -> Vec<MethodInfo> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|raw_name| MethodInfo {
                http_method: method_to_openapi(HttpMethod::Get),
                path: default_path(module_path, interface_name, &raw_name),
                operation_id: operation_id(module_path, interface_name, &raw_name),
                parameters: Vec::new(),
                request_body: None,
                response_schema: schema_for_type(&spec.ty),
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let raw_name = decl.0.clone();
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Get),
                            path: default_path(module_path, interface_name, &raw_name),
                            operation_id: operation_id(module_path, interface_name, &raw_name),
                            parameters: Vec::new(),
                            request_body: None,
                            response_schema: schema_for_type(&spec.ty),
                        });
                        let raw_setter = format!("set_{raw_name}");
                        let mut props = Map::new();
                        props.insert("value".to_string(), schema_for_type(&spec.ty));
                        let required = vec![Value::String("value".to_string())];
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Post),
                            path: default_path(module_path, interface_name, &raw_setter),
                            operation_id: operation_id(module_path, interface_name, &raw_setter),
                            parameters: Vec::new(),
                            request_body: body_schema(props, required),
                            response_schema: json!({ "type": "null" }),
                        });
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let raw_name = declarator.0.clone();
                    out.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Get),
                        path: default_path(module_path, interface_name, &raw_name),
                        operation_id: operation_id(module_path, interface_name, &raw_name),
                        parameters: Vec::new(),
                        request_body: None,
                        response_schema: schema_for_type(&spec.ty),
                    });
                    let raw_setter = format!("set_{raw_name}");
                    let mut props = Map::new();
                    props.insert("value".to_string(), schema_for_type(&spec.ty));
                    let required = vec![Value::String("value".to_string())];
                    out.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Post),
                        path: default_path(module_path, interface_name, &raw_setter),
                        operation_id: operation_id(module_path, interface_name, &raw_setter),
                        parameters: Vec::new(),
                        request_body: body_schema(props, required),
                        response_schema: json!({ "type": "null" }),
                    });
                }
            }
            out
        }
    }
}

fn body_schema(props: Map<String, Value>, required: Vec<Value>) -> Option<Value> {
    if props.is_empty() {
        return None;
    }
    Some(json!({
        "required": required,
        "content": {
            "application/json": {
                "schema": {
                    "type": "object",
                    "properties": props
                }
            }
        }
    }))
}

fn parameter_schema(location: &str, name: &str, schema: Value, required: bool) -> Value {
    json!({
        "name": name,
        "in": location,
        "required": required,
        "schema": schema,
    })
}

fn schema_for_struct(members: &[hir::Member]) -> Value {
    let mut props = Map::new();
    let mut required = Vec::new();
    for member in members {
        for decl in &member.ident {
            let name = declarator_name(decl);
            let schema = schema_for_decl(&member.ty, decl);
            props.insert(name.clone(), schema);
            required.push(Value::String(name));
        }
    }
    json!({
        "type": "object",
        "properties": props,
        "required": required,
    })
}

fn schema_for_union(def: &hir::UnionDef) -> Value {
    let mut variants = Vec::new();
    for case in &def.case {
        let decl = &case.element.value;
        let name = declarator_name(decl);
        let schema = schema_for_element(&case.element.ty, decl);
        let name_required = name.clone();
        variants.push(json!({
            "type": "object",
            "properties": {
                name: schema,
            },
            "required": [name_required],
        }));
    }
    json!({ "oneOf": variants })
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
            let size = xidl_parser::hir::const_expr_to_i64(&len.0);
            let mut array_schema = Map::new();
            array_schema.insert("type".to_string(), Value::String("array".to_string()));
            array_schema.insert("items".to_string(), schema);
            if let Some(size) = size {
                array_schema.insert("minItems".to_string(), Value::Number(size.into()));
                array_schema.insert("maxItems".to_string(), Value::Number(size.into()));
            }
            schema = Value::Object(array_schema);
        }
    }
    schema
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
            hir::SimpleTypeSpec::ScopedName(value) => schema_ref(&scoped_name_ref(value)),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let mut schema = json!({
                    "type": "array",
                    "items": schema_for_type(&seq.ty)
                });
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if let Some(obj) = schema.as_object_mut() {
                            obj.insert("minItems".to_string(), Value::Number(size.into()));
                            obj.insert("maxItems".to_string(), Value::Number(size.into()));
                        }
                    }
                }
                schema
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                json!({ "type": "string" })
            }
            hir::TemplateTypeSpec::FixedPtType(_) => {
                json!({ "type": "number", "format": "double" })
            }
            hir::TemplateTypeSpec::MapType(map) => json!({
                "type": "object",
                "additionalProperties": schema_for_type(&map.value)
            }),
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

fn scoped_name_ref(value: &hir::ScopedName) -> String {
    value.name.join(".")
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

fn operation_id(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

#[derive(Clone, Copy)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy)]
enum ParamSource {
    Path,
    Query,
    Body,
}

fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
    default_path: String,
) -> (HttpMethod, String) {
    for annotation in annotations {
        let Some(method) = method_from_annotation(annotation) else {
            continue;
        };
        let mut path = None;
        if let Some(params) = annotation_params(annotation) {
            let params = normalize_params(&params);
            path = params.get("path").cloned();
        }
        return (method, path.unwrap_or(default_path));
    }
    (default_method, default_path)
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    match name.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    let mut out = Vec::new();
    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            let value = trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string());
            out.push((key.trim().to_string(), unescape_param_value(&value)));
        }
    }
    out
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(buf.clone());
                }
                in_param = false;
            }
            _ => {
                if in_param {
                    buf.push(ch);
                }
            }
        }
    }

    out
}

fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

fn method_to_openapi(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "get".to_string(),
        HttpMethod::Post => "post".to_string(),
        HttpMethod::Put => "put".to_string(),
        HttpMethod::Patch => "patch".to_string(),
        HttpMethod::Delete => "delete".to_string(),
        HttpMethod::Head => "head".to_string(),
        HttpMethod::Options => "options".to_string(),
    }
}

fn error_schema_ref() -> Value {
    schema_ref("Error")
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn schema_for_constr_type(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> Value {
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
