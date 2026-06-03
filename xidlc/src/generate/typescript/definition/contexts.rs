use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TypesFileContext {
    pub(crate) blocks: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ClientFileContext {
    pub(crate) file_stem: String,
    pub(crate) helpers: Vec<String>,
    pub(crate) blocks: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ModuleContext {
    pub(crate) ident: String,
    pub(crate) blocks: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", content = "data")]
pub(crate) enum TsType {
    Primitive(String),
    ScopedName(String),
    Sequence(Box<TsType>),
    Map(Box<TsType>),
    Template { ident: String, args: Vec<TsType> },
    InlineStruct(Vec<FieldTypeContext>),
    InlineEnum(Vec<String>),
    Union(Vec<TsType>),
    Any,
    Void,
    AsyncIterable(Box<TsType>),
    Optional(Box<TsType>),
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", content = "data")]
pub(crate) enum ZodSchema {
    Primitive(String),
    ScopedName(String),
    Array {
        element: Box<ZodSchema>,
        length: Option<i64>,
    },
    Record(Box<ZodSchema>),
    Object(Vec<FieldZodContext>),
    Enum(Vec<String>),
    Union(Vec<ZodSchema>),
    Custom(TsType),
    Any,
    Never,
}

#[derive(Serialize, Clone)]
pub(crate) struct FieldTypeContext {
    pub(crate) prop: String,
    pub(crate) ty: TsType,
    pub(crate) optional: bool,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize, Clone)]
pub(crate) struct XjsonMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flatten: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) omitempty: Option<bool>,
}

#[derive(Serialize, Clone)]
pub(crate) struct FieldZodContext {
    pub(crate) prop: String,
    pub(crate) schema: ZodSchema,
    pub(crate) optional: bool,
    pub(crate) xjson_meta: Option<XjsonMeta>,
}

#[derive(Serialize)]
pub(crate) struct StructTypeContext {
    pub(crate) ident: String,
    pub(crate) extends: Option<Vec<TsType>>,
    pub(crate) fields: Vec<FieldTypeContext>,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct StructZodContext {
    pub(crate) ident: String,
    pub(crate) schema_name: String,
    pub(crate) fields: Vec<FieldZodContext>,
}

#[derive(Serialize)]
pub(crate) struct EnumTypeContext {
    pub(crate) ident: String,
    pub(crate) members: Vec<String>,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct EnumZodContext {
    pub(crate) ident: String,
    pub(crate) schema_name: String,
    pub(crate) values: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct UnionTypeContext {
    pub(crate) ident: String,
    pub(crate) variants: Vec<TsType>,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct UnionZodContext {
    pub(crate) ident: String,
    pub(crate) schema_name: String,
    pub(crate) variants: Vec<ZodSchema>,
}

#[derive(Serialize)]
pub(crate) struct TypedefTypeContext {
    pub(crate) name: String,
    pub(crate) type_expr: TsType,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct TypedefZodContext {
    pub(crate) name: String,
    pub(crate) schema_name: String,
    pub(crate) schema_expr: ZodSchema,
}

#[derive(Serialize, Clone)]
pub(crate) struct ParamDeclContext {
    pub(crate) prop: String,
    pub(crate) ty: TsType,
    pub(crate) schema: ZodSchema,
    pub(crate) optional: bool,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct RequestContext {
    pub(crate) name: String,
    pub(crate) params: Vec<ParamDeclContext>,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct RequestZodContext {
    pub(crate) name: String,
    pub(crate) schema_name: String,
    pub(crate) params: Vec<ParamDeclContext>,
}

#[derive(Serialize)]
pub(crate) struct ClientClassContext {
    pub(crate) client_name: String,
    pub(crate) methods: Vec<ClientMethodContext>,
}

#[derive(Serialize)]
pub(crate) struct ClientMethodContext {
    pub(crate) name: String,
    pub(crate) params: Vec<ClientParamContext>,
    pub(crate) return_ty: TsType,
    pub(crate) return_schema_ref: Option<String>,
    pub(crate) stream_item_ty: Option<TsType>,
    pub(crate) client_stream_item_ty: Option<TsType>,
    pub(crate) request_schema_ref: Option<String>,
    pub(crate) body_schema_ref: Option<String>,
    pub(crate) request_payload: Vec<RequestPayloadEntry>,
    pub(crate) path: String,
    pub(crate) http_method: String,
    pub(crate) path_params: Vec<PathParamContext>,
    pub(crate) query_params: Vec<QueryParamContext>,
    pub(crate) header_params: Vec<HeaderParamContext>,
    pub(crate) cookie_params: Vec<CookieParamContext>,
    pub(crate) body_params: Vec<BodyParamContext>,
    pub(crate) body_single: Option<BodyParamContext>,
    pub(crate) is_server_stream: bool,
    pub(crate) is_client_stream: bool,
    pub(crate) doc: Vec<String>,
}

#[derive(Serialize, Clone)]
pub(crate) struct ClientParamContext {
    pub(crate) name: String,
    pub(crate) ty: TsType,
}

#[derive(Serialize)]
pub(crate) struct RequestPayloadEntry {
    pub(crate) raw_name: String,
    pub(crate) name: String,
}

#[derive(Serialize)]
pub(crate) struct PathParamContext {
    pub(crate) template_name: String,
    pub(crate) catch_all: bool,
    pub(crate) access: String,
}

#[derive(Serialize)]
pub(crate) struct QueryParamContext {
    pub(crate) raw_name: String,
    pub(crate) access: String,
}

#[derive(Serialize)]
pub(crate) struct HeaderParamContext {
    pub(crate) raw_name: String,
    pub(crate) access: String,
    pub(crate) is_multi: bool,
}

#[derive(Serialize)]
pub(crate) struct CookieParamContext {
    pub(crate) raw_name: String,
    pub(crate) access: String,
    pub(crate) is_multi: bool,
}

#[derive(Serialize)]
pub(crate) struct BodyParamContext {
    pub(crate) raw_name: String,
    pub(crate) access: String,
}
