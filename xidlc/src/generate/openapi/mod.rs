use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem;
use utoipa::openapi::path::{
    HttpMethod as OpenApiHttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItem,
    PathsBuilder,
};
use utoipa::openapi::request_body::RequestBody;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{
    ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use utoipa::openapi::{
    Content, InfoBuilder, OpenApi, OpenApiBuilder, Ref, RefOr, Required, ResponsesBuilder,
};
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

use crate::jsonrpc::{Artifact, ArtifactFile};

pub(crate) struct OpenApiCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenApiCodegen {
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
        let openapi = render_openapi(&hir);
        let content = serde_json::to_string_pretty(&openapi)?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openapi.json".to_string(),
            content,
        })])
    }
}

pub fn render_openapi(spec: &hir::Specification) -> OpenApi {
    let mut ctx = OpenApiContext::default();
    ctx.collect_spec(spec, &[]);

    let mut components = utoipa::openapi::ComponentsBuilder::new();
    for (name, schema) in ctx.schemas {
        components = components.schema(name, schema);
    }

    let title = ctx.info_title.as_deref().unwrap_or("xidl");
    let version = ctx.info_version.as_deref().unwrap_or("0.1.0");

    OpenApiBuilder::new()
        .info(InfoBuilder::new().title(title).version(version).build())
        .paths(ctx.paths.build())
        .components(Some(components.build()))
        .build()
}

#[derive(Default)]
struct OpenApiContext {
    schemas: BTreeMap<String, RefOr<Schema>>,
    paths: PathsBuilder,
    info_title: Option<String>,
    info_version: Option<String>,
}

impl OpenApiContext {
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
            hir::Definition::Pragma(pragma) => {
                self.apply_pragma(pragma);
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
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new()
                        .schema_type(Type::String)
                        .enum_values(Some(values.collect::<Vec<_>>())),
                ));
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = schema_for_union(def);
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                self.schemas.insert(
                    name,
                    RefOr::T(Schema::from(
                        ObjectBuilder::new().schema_type(Type::Integer),
                    )),
                );
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                self.schemas.insert(
                    name,
                    RefOr::T(Schema::from(
                        ObjectBuilder::new().schema_type(Type::Integer),
                    )),
                );
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
            let MethodInfo {
                http_method,
                path,
                operation_id,
                parameters,
                request_body,
                response_schema,
            } = method;
            let mut operation = OperationBuilder::new()
                .operation_id(Some(operation_id))
                .responses(
                    ResponsesBuilder::new()
                        .response(
                            "200",
                            ResponseBuilder::new()
                                .description("OK")
                                .content("application/json", Content::new(Some(response_schema)))
                                .build(),
                        )
                        .response(
                            "500",
                            ResponseBuilder::new()
                                .description("Error")
                                .content("application/json", Content::new(Some(error_schema_ref())))
                                .build(),
                        )
                        .build(),
                );

            for parameter in parameters {
                operation = operation.parameter(parameter);
            }
            if let Some(request_body) = request_body {
                operation = operation.request_body(Some(request_body));
            }

            let paths = mem::take(&mut self.paths);
            self.paths = paths.path(path, PathItem::new(http_method, operation));
        }

        self.schemas.entry("Error".to_string()).or_insert_with(|| {
            RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .property("code", ObjectBuilder::new().schema_type(Type::Integer))
                    .required("code")
                    .property("msg", ObjectBuilder::new().schema_type(Type::String))
                    .required("msg"),
            ))
        });
    }
}

struct MethodInfo {
    http_method: OpenApiHttpMethod,
    path: String,
    operation_id: String,
    parameters: Vec<utoipa::openapi::path::Parameter>,
    request_body: Option<RequestBody>,
    response_schema: RefOr<Schema>,
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodInfo {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => {
            RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Null)))
        }
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
    let mut body_props = Vec::new();
    let mut body_required = Vec::new();

    for param in params {
        let raw_name = param.declarator.0.clone();
        let schema = schema_for_type(&param.ty);
        let optional = has_optional_annotation(&param.annotations);
        let source = if path_param_names.contains(&raw_name) {
            ParamSource::Path
        } else {
            default_source
        };
        match source {
            ParamSource::Path => {
                parameters.push(parameter_schema(ParameterIn::Path, &raw_name, schema, true))
            }
            ParamSource::Query => parameters.push(parameter_schema(
                ParameterIn::Query,
                &raw_name,
                schema,
                !optional,
            )),
            ParamSource::Body => {
                body_props.push((raw_name.clone(), schema));
                if !optional {
                    body_required.push(raw_name);
                }
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
                        let props = vec![("value".to_string(), schema_for_type(&spec.ty))];
                        let required = vec!["value".to_string()];
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Post),
                            path: default_path(module_path, interface_name, &raw_setter),
                            operation_id: operation_id(module_path, interface_name, &raw_setter),
                            parameters: Vec::new(),
                            request_body: body_schema(props, required),
                            response_schema: RefOr::T(Schema::from(
                                ObjectBuilder::new().schema_type(Type::Null),
                            )),
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
                    let props = vec![("value".to_string(), schema_for_type(&spec.ty))];
                    let required = vec!["value".to_string()];
                    out.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Post),
                        path: default_path(module_path, interface_name, &raw_setter),
                        operation_id: operation_id(module_path, interface_name, &raw_setter),
                        parameters: Vec::new(),
                        request_body: body_schema(props, required),
                        response_schema: RefOr::T(Schema::from(
                            ObjectBuilder::new().schema_type(Type::Null),
                        )),
                    });
                }
            }
            out
        }
    }
}

fn body_schema(props: Vec<(String, RefOr<Schema>)>, required: Vec<String>) -> Option<RequestBody> {
    if props.is_empty() {
        return None;
    }
    let schema = if props.len() == 1 {
        let (_, schema) = props.into_iter().next()?;
        schema
    } else {
        let mut object = ObjectBuilder::new().schema_type(Type::Object);
        for (name, schema) in props {
            object = object.property(name.clone(), schema);
        }
        for name in required {
            object = object.required(name);
        }
        RefOr::T(Schema::from(object))
    };

    let mut request_body = RequestBody::new();
    request_body
        .content
        .insert("application/json".to_string(), Content::new(Some(schema)));
    Some(request_body)
}

fn parameter_schema(
    location: ParameterIn,
    name: &str,
    schema: RefOr<Schema>,
    required: bool,
) -> utoipa::openapi::path::Parameter {
    let required = if required {
        Required::True
    } else {
        Required::False
    };
    ParameterBuilder::new()
        .name(name)
        .parameter_in(location)
        .required(required)
        .schema(Some(schema))
        .build()
}

fn schema_for_struct(members: &[hir::Member]) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Object);
    for member in members {
        let optional = member.is_optional();
        for decl in &member.ident {
            let name = declarator_name(decl);
            let schema = schema_for_decl(&member.ty, decl);
            object = object.property(name.clone(), schema);
            if !optional {
                object = object.required(name);
            }
        }
    }
    RefOr::T(Schema::from(object))
}

fn schema_for_union(def: &hir::UnionDef) -> RefOr<Schema> {
    let mut variants = Vec::new();
    for case in &def.case {
        let decl = &case.element.value;
        let name = declarator_name(decl);
        let schema = schema_for_element(&case.element.ty, decl);
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

fn schema_for_type(ty: &hir::TypeSpec) -> RefOr<Schema> {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_schema(value),
            hir::SimpleTypeSpec::FloatingPtType => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Number)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
            )),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
            }
            hir::SimpleTypeSpec::Boolean => RefOr::T(Schema::from(
                ObjectBuilder::new().schema_type(Type::Boolean),
            )),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => RefOr::T(Schema::from(ObjectBuilder::new())),
            hir::SimpleTypeSpec::ScopedName(value) => schema_ref(&scoped_name_ref(value)),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let mut schema = ArrayBuilder::new().items(schema_for_type(&seq.ty));
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if size >= 0 {
                            let size = size as usize;
                            schema = schema.min_items(Some(size)).max_items(Some(size));
                        }
                    }
                }
                RefOr::T(Schema::from(schema))
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
            }
            hir::TemplateTypeSpec::FixedPtType(_) => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Number)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
            )),
            hir::TemplateTypeSpec::MapType(map) => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .additional_properties(Some(schema_for_type(&map.value))),
            )),
            hir::TemplateTypeSpec::TemplateType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Object)))
            }
        },
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
        _ => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)));
        }
    }
    RefOr::T(Schema::from(object))
}

fn schema_ref(name: &str) -> RefOr<Schema> {
    RefOr::Ref(Ref::from_schema_name(name))
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
            let params = normalize_params(params);
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

fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("optional"))
            .unwrap_or(false)
    })
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

fn method_to_openapi(method: HttpMethod) -> OpenApiHttpMethod {
    match method {
        HttpMethod::Get => OpenApiHttpMethod::Get,
        HttpMethod::Post => OpenApiHttpMethod::Post,
        HttpMethod::Put => OpenApiHttpMethod::Put,
        HttpMethod::Patch => OpenApiHttpMethod::Patch,
        HttpMethod::Delete => OpenApiHttpMethod::Delete,
        HttpMethod::Head => OpenApiHttpMethod::Head,
        HttpMethod::Options => OpenApiHttpMethod::Options,
    }
}

fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn schema_for_constr_type(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> RefOr<Schema> {
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
