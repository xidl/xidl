use crate::error::IdlcResult;
use crate::generate::typescript::{TsMode, TypescriptRenderOutput, TypescriptRenderer};
use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::HashSet;
use xidl_parser::hir;

pub fn render_typescript(
    spec: &hir::Specification,
    file_stem: &str,
    renderer: &TypescriptRenderer,
    mode: TsMode,
) -> IdlcResult<TypescriptRenderOutput> {
    let mut generator = TsGenerator::new(file_stem.to_string());
    generator.render(spec, renderer, mode)
}

struct TsGenerator {
    file_stem: String,
}

impl TsGenerator {
    fn new(file_stem: String) -> Self {
        Self { file_stem }
    }

    fn render(
        &mut self,
        spec: &hir::Specification,
        renderer: &TypescriptRenderer,
        mode: TsMode,
    ) -> IdlcResult<TypescriptRenderOutput> {
        let blocks = render_module_body(&spec.0, &[], renderer, mode)?;

        let types = renderer.render_template(
            "types.d.ts.j2",
            &TypesFileContext {
                blocks: blocks.types,
            },
        )?;
        let zod =
            renderer.render_template("zod.ts.j2", &TypesFileContext { blocks: blocks.zod })?;

        let client = if mode.allows_interfaces() {
            let helpers = renderer.render_template("client_helpers.ts.j2", &())?;
            renderer.render_template(
                "client.ts.j2",
                &ClientFileContext {
                    file_stem: self.file_stem.clone(),
                    helpers: vec![helpers],
                    blocks: blocks.client,
                },
            )?
        } else {
            String::new()
        };

        Ok(TypescriptRenderOutput { types, zod, client })
    }
}

#[derive(Default)]
struct TsRenderOutput {
    types: Vec<String>,
    zod: Vec<String>,
    client: Vec<String>,
}

impl TsRenderOutput {
    fn extend(&mut self, other: TsRenderOutput) {
        self.types.extend(other.types);
        self.zod.extend(other.zod);
        self.client.extend(other.client);
    }

    fn is_empty(&self) -> bool {
        self.types.is_empty() && self.zod.is_empty() && self.client.is_empty()
    }
}

#[derive(Serialize)]
struct TypesFileContext {
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct ClientFileContext {
    file_stem: String,
    helpers: Vec<String>,
    blocks: Vec<String>,
}

fn render_module_body(
    defs: &[hir::Definition],
    module_path: &[String],
    renderer: &TypescriptRenderer,
    mode: TsMode,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    let mut module_order = Vec::new();
    let mut module_map: std::collections::HashMap<String, Vec<TsRenderOutput>> =
        std::collections::HashMap::new();

    for def in defs {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                let body = render_module_body(&module.definition, &next_path, renderer, mode)?;
                if !body.is_empty() {
                    let entry = module_map.entry(module.ident.clone()).or_insert_with(|| {
                        module_order.push(module.ident.clone());
                        Vec::new()
                    });
                    entry.push(body);
                }
            }
            hir::Definition::ConstrTypeDcl(constr) => {
                if mode.allows_types() {
                    out.extend(render_constr_type(constr, module_path, renderer)?);
                }
            }
            hir::Definition::TypeDcl(ty) => {
                if mode.allows_types() {
                    out.extend(render_type_dcl(ty, module_path, renderer)?);
                }
            }
            hir::Definition::ExceptDcl(except) => {
                if mode.allows_types() {
                    out.extend(render_exception(except, module_path, renderer)?);
                }
            }
            hir::Definition::InterfaceDcl(interface) => {
                if mode.allows_interfaces() {
                    out.extend(render_interface(interface, module_path, renderer)?);
                }
            }
            hir::Definition::ConstDcl(_) | hir::Definition::Pragma(_) => {}
        }
    }

    for name in module_order {
        let modules = module_map.remove(&name).unwrap_or_default();
        let body = merge_blocks(&modules);
        let ident = ts_ident(&name);
        let types = renderer.render_template(
            "module.ts.j2",
            &ModuleContext {
                ident: ident.clone(),
                body: indent_block(&body.types.join("\n"), 1),
            },
        )?;
        let zod = renderer.render_template(
            "module.ts.j2",
            &ModuleContext {
                ident: ident.clone(),
                body: indent_block(&body.zod.join("\n"), 1),
            },
        )?;
        let client = renderer.render_template(
            "module.ts.j2",
            &ModuleContext {
                ident,
                body: indent_block(&body.client.join("\n"), 1),
            },
        )?;
        out.types.push(types);
        out.zod.push(zod);
        out.client.push(client);
    }

    Ok(out)
}

fn merge_blocks(blocks: &[TsRenderOutput]) -> TsRenderOutput {
    let mut out = TsRenderOutput::default();
    for block in blocks {
        out.types.extend(block.types.iter().cloned());
        out.zod.extend(block.zod.iter().cloned());
        out.client.extend(block.client.iter().cloned());
    }
    out
}

#[derive(Serialize)]
struct ModuleContext {
    ident: String,
    body: String,
}

fn render_constr_type(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            out.extend(render_struct(def, module_path, renderer)?);
        }
        hir::ConstrTypeDcl::EnumDcl(def) => {
            out.extend(render_enum(def, renderer)?);
        }
        hir::ConstrTypeDcl::UnionDef(def) => {
            out.extend(render_union(def, module_path, renderer)?);
        }
        hir::ConstrTypeDcl::BitsetDcl(def) => {
            out.extend(render_bit_number(&def.ident, renderer)?);
        }
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            out.extend(render_bit_number(&def.ident, renderer)?);
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
    }
    Ok(out)
}

fn render_exception(
    except: &hir::ExceptDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let def = hir::StructDcl {
        annotations: Vec::new(),
        ident: except.ident.clone(),
        parent: Vec::new(),
        member: except.member.clone(),
    };
    render_struct(&def, module_path, renderer)
}

fn render_type_dcl(
    ty: &hir::TypeDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let mut out = TsRenderOutput::default();
    match &ty.decl {
        hir::TypeDclInner::ConstrTypeDcl(constr) => {
            out.extend(render_constr_type(constr, module_path, renderer)?);
        }
        hir::TypeDclInner::TypedefDcl(typedef) => {
            for decl in &typedef.decl {
                let name = ts_ident(declarator_name(decl));
                let type_expr = match &typedef.ty {
                    hir::TypedefType::TypeSpec(spec) => {
                        ts_type_for_decl(spec, decl, module_path, TypeRefTarget::Types)
                    }
                    hir::TypedefType::ConstrTypeDcl(constr) => {
                        let base =
                            ts_type_for_constr_inline(constr, module_path, TypeRefTarget::Types);
                        apply_array_ts(base, decl)
                    }
                };
                let types = renderer.render_template(
                    "typedef.d.ts.j2",
                    &TypedefTypeContext {
                        name: name.clone(),
                        type_expr,
                    },
                )?;
                out.types.push(types);

                let schema_expr = match &typedef.ty {
                    hir::TypedefType::TypeSpec(spec) => {
                        zod_schema_for_decl(spec, decl, module_path)
                    }
                    hir::TypedefType::ConstrTypeDcl(constr) => {
                        let base = zod_schema_for_constr_inline(constr, module_path);
                        apply_array_zod(base, decl)
                    }
                };
                let schema_name = format!("{name}Schema");
                let zod = renderer.render_template(
                    "typedef.zod.ts.j2",
                    &TypedefZodContext {
                        name,
                        schema_name,
                        schema_expr,
                    },
                )?;
                out.zod.push(zod);
            }
        }
        hir::TypeDclInner::NativeDcl(native) => {
            let name = ts_ident(&native.decl.0);
            let types = renderer.render_template(
                "typedef.d.ts.j2",
                &TypedefTypeContext {
                    name: name.clone(),
                    type_expr: "unknown".to_string(),
                },
            )?;
            out.types.push(types);
            let schema_name = format!("{name}Schema");
            let zod = renderer.render_template(
                "typedef.zod.ts.j2",
                &TypedefZodContext {
                    name,
                    schema_name,
                    schema_expr: "z.unknown()".to_string(),
                },
            )?;
            out.zod.push(zod);
        }
    }
    Ok(out)
}

fn render_interface(
    interface: &hir::InterfaceDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let def = match &interface.decl {
        hir::InterfaceDclInner::InterfaceDef(def) => def,
        _ => return Ok(TsRenderOutput::default()),
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

    let mut out = TsRenderOutput::default();

    for method in &methods {
        if let Some(request_name) = &method.request_name {
            let params = method
                .params
                .iter()
                .map(|param| ParamDeclContext {
                    prop: ts_prop_name(&param.raw_name),
                    ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Types),
                    schema: zod_schema_for_type_spec(&param.ty, module_path),
                })
                .collect::<Vec<_>>();
            let types = renderer.render_template(
                "request.d.ts.j2",
                &RequestContext {
                    name: request_name.clone(),
                    params: params.clone(),
                },
            )?;
            let schema_name = format!("{request_name}Schema");
            let zod = renderer.render_template(
                "request.zod.ts.j2",
                &RequestZodContext {
                    name: request_name.clone(),
                    schema_name,
                    params,
                },
            )?;
            out.types.push(types);
            out.zod.push(zod);
        }
    }

    let client = renderer.render_template(
        "client_class.ts.j2",
        &ClientClassContext {
            client_name: format!("{}Client", ts_ident(&def.header.ident)),
            methods: methods
                .iter()
                .map(|method| method.to_template(module_path))
                .collect(),
        },
    )?;
    out.client.push(client);

    Ok(out)
}

fn render_struct(
    def: &hir::StructDcl,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let parents = def
        .parent
        .iter()
        .map(|parent| ts_scoped_name(parent, module_path, TypeRefTarget::Types))
        .collect::<Vec<_>>();
    let extends = if parents.is_empty() {
        None
    } else {
        Some(parents.join(", "))
    };

    let fields = struct_fields(&def.member, module_path);
    let types = renderer.render_template(
        "struct.d.ts.j2",
        &StructTypeContext {
            ident: ident.clone(),
            extends,
            fields: fields
                .iter()
                .map(|field| FieldTypeContext {
                    prop: field.prop.clone(),
                    ty: field.ty.clone(),
                })
                .collect(),
        },
    )?;
    let schema_name = format!("{ident}Schema");
    let zod = renderer.render_template(
        "struct.zod.ts.j2",
        &StructZodContext {
            ident,
            schema_name,
            fields: fields
                .into_iter()
                .map(|field| FieldZodContext {
                    prop: field.prop,
                    schema: field.schema,
                })
                .collect(),
        },
    )?;

    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

fn render_enum(def: &hir::EnumDcl, renderer: &TypescriptRenderer) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let members = def
        .member
        .iter()
        .map(|value| format!("\"{}\"", value.ident))
        .collect::<Vec<_>>();
    let union = if members.is_empty() {
        "never".to_string()
    } else {
        members.join(" | ")
    };
    let types = renderer.render_template(
        "enum.d.ts.j2",
        &EnumTypeContext {
            ident: ident.clone(),
            union,
        },
    )?;
    let schema_name = format!("{ident}Schema");
    let zod = renderer.render_template(
        "enum.zod.ts.j2",
        &EnumZodContext {
            ident,
            schema_name,
            values: members,
        },
    )?;

    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

fn render_union(
    def: &hir::UnionDef,
    module_path: &[String],
    renderer: &TypescriptRenderer,
) -> IdlcResult<TsRenderOutput> {
    let ident = ts_ident(&def.ident);
    let mut variants = Vec::new();
    let mut schema_variants = Vec::new();
    for case in &def.case {
        let name = declarator_name(&case.element.value);
        let prop = ts_prop_name(name);
        let ty = ts_type_for_element(
            &case.element.ty,
            &case.element.value,
            module_path,
            TypeRefTarget::Types,
        );
        variants.push(format!("{{ {prop}: {ty} }}"));
        let schema = zod_schema_for_element(&case.element.ty, &case.element.value, module_path);
        schema_variants.push(format!("z.object({{ {prop}: {schema} }})"));
    }

    let union = if variants.is_empty() {
        "never".to_string()
    } else {
        variants.join(" | ")
    };
    let types = renderer.render_template(
        "union.d.ts.j2",
        &UnionTypeContext {
            ident: ident.clone(),
            union,
        },
    )?;
    let schema_name = format!("{ident}Schema");
    let zod = renderer.render_template(
        "union.zod.ts.j2",
        &UnionZodContext {
            ident,
            schema_name,
            variants: schema_variants,
        },
    )?;

    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

fn render_bit_number(ident: &str, renderer: &TypescriptRenderer) -> IdlcResult<TsRenderOutput> {
    let name = ts_ident(ident);
    let types = renderer.render_template(
        "typedef.d.ts.j2",
        &TypedefTypeContext {
            name: name.clone(),
            type_expr: "number".to_string(),
        },
    )?;
    let schema_name = format!("{name}Schema");
    let zod = renderer.render_template(
        "typedef.zod.ts.j2",
        &TypedefZodContext {
            name,
            schema_name,
            schema_expr: "z.number().int()".to_string(),
        },
    )?;
    Ok(TsRenderOutput {
        types: vec![types],
        zod: vec![zod],
        client: Vec::new(),
    })
}

#[derive(Serialize)]
struct FieldTypeContext {
    prop: String,
    ty: String,
}

#[derive(Serialize)]
struct FieldZodContext {
    prop: String,
    schema: String,
}

#[derive(Serialize)]
struct StructTypeContext {
    ident: String,
    extends: Option<String>,
    fields: Vec<FieldTypeContext>,
}

#[derive(Serialize)]
struct StructZodContext {
    ident: String,
    schema_name: String,
    fields: Vec<FieldZodContext>,
}

#[derive(Serialize)]
struct EnumTypeContext {
    ident: String,
    union: String,
}

#[derive(Serialize)]
struct EnumZodContext {
    ident: String,
    schema_name: String,
    values: Vec<String>,
}

#[derive(Serialize)]
struct UnionTypeContext {
    ident: String,
    union: String,
}

#[derive(Serialize)]
struct UnionZodContext {
    ident: String,
    schema_name: String,
    variants: Vec<String>,
}

#[derive(Serialize)]
struct TypedefTypeContext {
    name: String,
    type_expr: String,
}

#[derive(Serialize)]
struct TypedefZodContext {
    name: String,
    schema_name: String,
    schema_expr: String,
}

#[derive(Serialize, Clone)]
struct ParamDeclContext {
    prop: String,
    ty: String,
    schema: String,
}

#[derive(Serialize)]
struct RequestContext {
    name: String,
    params: Vec<ParamDeclContext>,
}

#[derive(Serialize)]
struct RequestZodContext {
    name: String,
    schema_name: String,
    params: Vec<ParamDeclContext>,
}

#[derive(Serialize)]
struct ClientClassContext {
    client_name: String,
    methods: Vec<ClientMethodContext>,
}

#[derive(Serialize)]
struct ClientMethodContext {
    name: String,
    params: Vec<ClientParamContext>,
    return_ty: String,
    request_schema_ref: Option<String>,
    request_payload: Vec<RequestPayloadEntry>,
    path: String,
    http_method: String,
    path_params: Vec<PathParamContext>,
    query_params: Vec<QueryParamContext>,
    body_params: Vec<BodyParamContext>,
}

#[derive(Serialize)]
struct ClientParamContext {
    name: String,
    ty: String,
}

#[derive(Serialize)]
struct RequestPayloadEntry {
    raw_name: String,
    name: String,
}

#[derive(Serialize)]
struct PathParamContext {
    raw_name: String,
    access: String,
}

#[derive(Serialize)]
struct QueryParamContext {
    raw_name: String,
    access: String,
}

#[derive(Serialize)]
struct BodyParamContext {
    raw_name: String,
    access: String,
}

#[derive(Clone)]
struct ParamInfo {
    name: String,
    raw_name: String,
    ty: hir::TypeSpec,
}

#[derive(Clone)]
struct MethodInfo {
    name: String,
    params: Vec<ParamInfo>,
    ret: ReturnType,
    http_method: String,
    path: String,
    request_name: Option<String>,
    request_schema_ref: Option<String>,
    path_params: Vec<ParamInfo>,
    query_params: Vec<ParamInfo>,
    body_params: Vec<ParamInfo>,
}

impl MethodInfo {
    fn to_template(&self, module_path: &[String]) -> ClientMethodContext {
        let params = self
            .params
            .iter()
            .map(|param| ClientParamContext {
                name: param.name.clone(),
                ty: ts_type_for_type_spec(&param.ty, module_path, TypeRefTarget::Client),
            })
            .collect::<Vec<_>>();
        let return_ty = if self.ret.is_void {
            "void".to_string()
        } else {
            ts_type_for_type_spec(
                self.ret.ty.as_ref().expect("return type"),
                module_path,
                TypeRefTarget::Client,
            )
        };
        let request_payload = self
            .params
            .iter()
            .map(|param| RequestPayloadEntry {
                raw_name: param.raw_name.clone(),
                name: param.name.clone(),
            })
            .collect::<Vec<_>>();
        ClientMethodContext {
            name: self.name.clone(),
            params,
            return_ty,
            request_schema_ref: self.request_schema_ref.clone(),
            request_payload,
            path: self.path.clone(),
            http_method: self.http_method.clone(),
            path_params: self
                .path_params
                .iter()
                .map(|param| PathParamContext {
                    raw_name: param.raw_name.clone(),
                    access: parsed_access(self, &param.raw_name),
                })
                .collect(),
            query_params: self
                .query_params
                .iter()
                .map(|param| QueryParamContext {
                    raw_name: param.raw_name.clone(),
                    access: parsed_access(self, &param.raw_name),
                })
                .collect(),
            body_params: self
                .body_params
                .iter()
                .map(|param| BodyParamContext {
                    raw_name: param.raw_name.clone(),
                    access: parsed_access(self, &param.raw_name),
                })
                .collect(),
        }
    }
}

#[derive(Clone)]
struct ReturnType {
    is_void: bool,
    ty: Option<hir::TypeSpec>,
}

impl ReturnType {
    fn void() -> Self {
        Self {
            is_void: true,
            ty: None,
        }
    }

    fn new(ty: hir::TypeSpec) -> Self {
        Self {
            is_void: false,
            ty: Some(ty),
        }
    }
}

#[derive(Clone, Copy)]
enum TypeRefTarget {
    Types,
    Client,
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

fn struct_fields(members: &[hir::Member], module_path: &[String]) -> Vec<FieldDecl> {
    let mut fields = Vec::new();
    for member in members {
        for decl in &member.ident {
            let name = declarator_name(decl);
            fields.push(FieldDecl {
                prop: ts_prop_name(name),
                ty: ts_type_for_decl(&member.ty, decl, module_path, TypeRefTarget::Types),
                schema: zod_schema_for_decl(&member.ty, decl, module_path),
            });
        }
    }
    fields
}

struct FieldDecl {
    prop: String,
    ty: String,
    schema: String,
}

fn render_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodInfo {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => ReturnType::void(),
        hir::OpTypeSpec::TypeSpec(ty) => ReturnType::new(ty.clone()),
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

    let mut param_list = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut body_params = Vec::new();

    for param in params {
        let name = ts_ident(&param.declarator.0);
        let raw_name = param.declarator.0.clone();
        let ty = param.ty.clone();
        let source = if path_param_names.contains(&param.declarator.0) {
            ParamSource::Path
        } else {
            default_source
        };
        let info = ParamInfo { name, raw_name, ty };
        param_list.push(info.clone());
        match source {
            ParamSource::Path => path_params.push(info),
            ParamSource::Query => query_params.push(info),
            ParamSource::Body => body_params.push(info),
        }
    }

    let method_name = ts_ident(&op.ident);
    let request_name = if param_list.is_empty() {
        None
    } else {
        Some(format!(
            "{}Request",
            method_struct_prefix(interface_name, &op.ident)
        ))
    };
    let request_schema_ref = request_name.as_ref().map(|name| {
        let full = scoped_name(module_path, name);
        format!("zodSchemas.{full}Schema")
    });
    MethodInfo {
        name: method_name,
        params: param_list,
        ret,
        http_method: method_http_code(method),
        path,
        request_name,
        request_schema_ref,
        path_params,
        query_params,
        body_params,
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
            .map(|names| MethodInfo {
                name: ts_ident(&names.raw),
                params: Vec::new(),
                ret: ReturnType::new(spec.ty.clone()),
                http_method: method_http_code(HttpMethod::Get),
                path: default_path(module_path, interface_name, &names.raw),
                request_name: None,
                request_schema_ref: None,
                path_params: Vec::new(),
                query_params: Vec::new(),
                body_params: Vec::new(),
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let raw_name = decl.0.clone();
                        let getter = MethodInfo {
                            name: ts_ident(&raw_name),
                            params: Vec::new(),
                            ret: ReturnType::new(spec.ty.clone()),
                            http_method: method_http_code(HttpMethod::Get),
                            path: default_path(module_path, interface_name, &raw_name),
                            request_name: None,
                            request_schema_ref: None,
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: Vec::new(),
                        };
                        out.push(getter);

                        let setter_raw = format!("set_{raw_name}");
                        let param = ParamInfo {
                            name: "value".to_string(),
                            raw_name: "value".to_string(),
                            ty: spec.ty.clone(),
                        };
                        let request_name = Some(format!(
                            "{}Request",
                            method_struct_prefix(interface_name, &setter_raw)
                        ));
                        let request_schema_ref = request_name.as_ref().map(|name| {
                            let full = scoped_name(module_path, name);
                            format!("zodSchemas.{full}Schema")
                        });
                        let setter = MethodInfo {
                            name: ts_ident(&setter_raw),
                            params: vec![param.clone()],
                            ret: ReturnType::void(),
                            http_method: method_http_code(HttpMethod::Post),
                            path: default_path(module_path, interface_name, &setter_raw),
                            request_name,
                            request_schema_ref,
                            path_params: Vec::new(),
                            query_params: Vec::new(),
                            body_params: vec![param],
                        };
                        out.push(setter);
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let raw_name = declarator.0.clone();
                    let getter = MethodInfo {
                        name: ts_ident(&raw_name),
                        params: Vec::new(),
                        ret: ReturnType::new(spec.ty.clone()),
                        http_method: method_http_code(HttpMethod::Get),
                        path: default_path(module_path, interface_name, &raw_name),
                        request_name: None,
                        request_schema_ref: None,
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: Vec::new(),
                    };
                    out.push(getter);

                    let setter_raw = format!("set_{raw_name}");
                    let param = ParamInfo {
                        name: "value".to_string(),
                        raw_name: "value".to_string(),
                        ty: spec.ty.clone(),
                    };
                    let request_name = Some(format!(
                        "{}Request",
                        method_struct_prefix(interface_name, &setter_raw)
                    ));
                    let request_schema_ref = request_name.as_ref().map(|name| {
                        let full = scoped_name(module_path, name);
                        format!("zodSchemas.{full}Schema")
                    });
                    let setter = MethodInfo {
                        name: ts_ident(&setter_raw),
                        params: vec![param.clone()],
                        ret: ReturnType::void(),
                        http_method: method_http_code(HttpMethod::Post),
                        path: default_path(module_path, interface_name, &setter_raw),
                        request_name,
                        request_schema_ref,
                        path_params: Vec::new(),
                        query_params: Vec::new(),
                        body_params: vec![param],
                    };
                    out.push(setter);
                }
            }
            out
        }
    }
}

fn parsed_access(method: &MethodInfo, raw_name: &str) -> String {
    if method.request_schema_ref.is_some() {
        format!("parsed[\"{raw_name}\"]")
    } else {
        ts_ident(raw_name)
    }
}

fn ts_type_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let base = ts_type_for_type_spec(ty, module_path, target);
    apply_array_ts(base, decl)
}

fn ts_type_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let base = match ty {
        hir::ElementSpecTy::TypeSpec(spec) => ts_type_for_type_spec(spec, module_path, target),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => {
            ts_type_for_constr_inline(constr, module_path, target)
        }
    };
    apply_array_ts(base, decl)
}

fn ts_type_for_constr_inline(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let mut fields = Vec::new();
            for member in &def.member {
                for decl in &member.ident {
                    let name = ts_prop_name(declarator_name(decl));
                    let ty = ts_type_for_decl(&member.ty, decl, module_path, target);
                    fields.push(format!("{name}: {ty}"));
                }
            }
            format!("{{ {} }}", fields.join(", "))
        }
        hir::ConstrTypeDcl::EnumDcl(def) => def
            .member
            .iter()
            .map(|value| format!("\"{}\"", value.ident))
            .collect::<Vec<_>>()
            .join(" | "),
        hir::ConstrTypeDcl::UnionDef(def) => {
            let mut variants = Vec::new();
            for case in &def.case {
                let name = ts_prop_name(declarator_name(&case.element.value));
                let ty =
                    ts_type_for_element(&case.element.ty, &case.element.value, module_path, target);
                variants.push(format!("{{ {name}: {ty} }}"));
            }
            if variants.is_empty() {
                "never".to_string()
            } else {
                variants.join(" | ")
            }
        }
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => {
            "number".to_string()
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            "unknown".to_string()
        }
    }
}

fn zod_schema_for_decl(
    ty: &hir::TypeSpec,
    decl: &hir::Declarator,
    module_path: &[String],
) -> String {
    let base = zod_schema_for_type_spec(ty, module_path);
    apply_array_zod(base, decl)
}

fn zod_schema_for_element(
    ty: &hir::ElementSpecTy,
    decl: &hir::Declarator,
    module_path: &[String],
) -> String {
    let base = match ty {
        hir::ElementSpecTy::TypeSpec(spec) => zod_schema_for_type_spec(spec, module_path),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => {
            zod_schema_for_constr_inline(constr, module_path)
        }
    };
    apply_array_zod(base, decl)
}

fn zod_schema_for_constr_inline(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> String {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let mut fields = Vec::new();
            for member in &def.member {
                for decl in &member.ident {
                    let name = ts_prop_name(declarator_name(decl));
                    let schema = zod_schema_for_decl(&member.ty, decl, module_path);
                    fields.push(format!("{name}: {schema}"));
                }
            }
            format!("z.object({{ {} }})", fields.join(", "))
        }
        hir::ConstrTypeDcl::EnumDcl(def) => {
            let values = def
                .member
                .iter()
                .map(|value| format!("\"{}\"", value.ident))
                .collect::<Vec<_>>();
            if values.is_empty() {
                "z.never()".to_string()
            } else {
                format!("z.enum([{}])", values.join(", "))
            }
        }
        hir::ConstrTypeDcl::UnionDef(def) => {
            let mut variants = Vec::new();
            for case in &def.case {
                let name = ts_prop_name(declarator_name(&case.element.value));
                let schema =
                    zod_schema_for_element(&case.element.ty, &case.element.value, module_path);
                variants.push(format!("z.object({{ {name}: {schema} }})"));
            }
            if variants.is_empty() {
                "z.never()".to_string()
            } else if variants.len() == 1 {
                variants[0].clone()
            } else {
                format!("z.union([{}])", variants.join(", "))
            }
        }
        hir::ConstrTypeDcl::BitsetDcl(_) | hir::ConstrTypeDcl::BitmaskDcl(_) => {
            "z.number().int()".to_string()
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {
            "z.unknown()".to_string()
        }
    }
}

fn ts_type_for_type_spec(
    ty: &hir::TypeSpec,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(_) | hir::SimpleTypeSpec::FloatingPtType => {
                "number".to_string()
            }
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                "string".to_string()
            }
            hir::SimpleTypeSpec::Boolean => "boolean".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "unknown".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => ts_scoped_name(value, module_path, target),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let inner = ts_type_for_type_spec(&seq.ty, module_path, target);
                format!("Array<{inner}>")
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "string".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "number".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                let value = ts_type_for_type_spec(&map.value, module_path, target);
                format!("Record<string, {value}>")
            }
            hir::TemplateTypeSpec::TemplateType(value) => {
                ts_template_type(value, module_path, target)
            }
        },
    }
}

fn zod_schema_for_type_spec(ty: &hir::TypeSpec, module_path: &[String]) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_schema(value),
            hir::SimpleTypeSpec::FloatingPtType => "z.number()".to_string(),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                "z.string()".to_string()
            }
            hir::SimpleTypeSpec::Boolean => "z.boolean()".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "z.unknown()".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => zod_schema_ref(value, module_path),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let inner = zod_schema_for_type_spec(&seq.ty, module_path);
                let mut schema = format!("z.array({inner})");
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if size >= 0 {
                            schema = format!("{schema}.length({size})");
                        }
                    }
                }
                schema
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "z.string()".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "z.number()".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                let value = zod_schema_for_type_spec(&map.value, module_path);
                format!("z.record({value})")
            }
            hir::TemplateTypeSpec::TemplateType(value) => {
                let ty = ts_template_type(value, module_path, TypeRefTarget::Types);
                format!("z.custom<{ty}>()")
            }
        },
    }
}

fn ts_template_type(
    value: &hir::TemplateType,
    module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let args = value
        .args
        .iter()
        .map(|arg| ts_type_for_type_spec(arg, module_path, target))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{args}>", ts_ident(&value.ident))
}

fn apply_array_ts(mut base: String, decl: &hir::Declarator) -> String {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for _ in &array.len {
            base = format!("Array<{base}>");
        }
    }
    base
}

fn apply_array_zod(mut base: String, decl: &hir::Declarator) -> String {
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            let mut wrapped = format!("z.array({base})");
            if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                if size >= 0 {
                    wrapped = format!("{wrapped}.length({size})");
                }
            }
            base = wrapped;
        }
    }
    base
}

fn ts_scoped_name(
    value: &hir::ScopedName,
    _module_path: &[String],
    target: TypeRefTarget,
) -> String {
    let parts = value
        .name
        .iter()
        .map(|part| ts_ident(part))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "unknown".to_string();
    }
    let name = parts.join(".");
    match target {
        TypeRefTarget::Types => name,
        TypeRefTarget::Client => format!("types.{name}"),
    }
}

fn zod_schema_ref(value: &hir::ScopedName, _module_path: &[String]) -> String {
    let mut parts = value
        .name
        .iter()
        .map(|part| ts_ident(part))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "z.unknown()".to_string();
    }
    let last = parts.pop().unwrap();
    parts.push(format!("{last}Schema"));
    parts.join(".")
}

fn integer_schema(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::U64
        | hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar => "z.number().int().nonnegative()".to_string(),
        _ => "z.number().int()".to_string(),
    }
}

fn ts_ident(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        let valid = if idx == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        if valid {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || is_ts_keyword(&out) {
        format!("_{out}")
    } else {
        out
    }
}

fn ts_prop_name(value: &str) -> String {
    if is_valid_ts_ident(value) && !is_ts_keyword(value) {
        value.to_string()
    } else {
        format!("\"{}\"", value)
    }
}

fn is_valid_ts_ident(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }
    true
}

fn is_ts_keyword(value: &str) -> bool {
    matches!(
        value,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "as"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
            | "any"
            | "boolean"
            | "constructor"
            | "declare"
            | "get"
            | "module"
            | "require"
            | "number"
            | "set"
            | "string"
            | "symbol"
            | "type"
            | "from"
            | "of"
    )
}

fn declarator_name(decl: &hir::Declarator) -> &str {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => &simple.0,
        hir::Declarator::ArrayDeclarator(array) => &array.ident,
    }
}

fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}

fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ts_ident(ident)
    } else {
        let mut parts = module_path
            .iter()
            .map(|part| ts_ident(part))
            .collect::<Vec<_>>();
        parts.push(ts_ident(ident));
        parts.join(".")
    }
}

fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
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

fn normalize_params(params: &hir::AnnotationParams) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
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

fn method_http_code(method: HttpMethod) -> String {
    match method {
        HttpMethod::Get => "GET".to_string(),
        HttpMethod::Post => "POST".to_string(),
        HttpMethod::Put => "PUT".to_string(),
        HttpMethod::Patch => "PATCH".to_string(),
        HttpMethod::Delete => "DELETE".to_string(),
        HttpMethod::Head => "HEAD".to_string(),
        HttpMethod::Options => "OPTIONS".to_string(),
    }
}

struct AttrNames {
    raw: String,
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrNames> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrNames {
            raw: decl.0.clone(),
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn indent_block(value: &str, level: usize) -> String {
    let indent = "    ".repeat(level);
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
