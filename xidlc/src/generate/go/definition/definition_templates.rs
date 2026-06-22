use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct ConstTemplate {
    pub(super) name: String,
    pub(super) ty: String,
    pub(super) value: String,
}

#[derive(Serialize)]
pub(super) struct TypeDeclTemplate {
    pub(super) name: String,
    pub(super) ty: String,
    pub(super) alias: bool,
}

#[derive(Serialize)]
pub(super) struct FieldTemplate {
    pub(super) name: String,
    pub(super) ty: String,
    pub(super) tag: String,
}

#[derive(Serialize)]
pub(super) struct StructTemplate {
    pub(super) name: String,
    pub(super) fields: String,
}

#[derive(Serialize)]
pub(super) struct EmptyStructTemplate {
    pub(super) name: String,
}

#[derive(Serialize)]
pub(super) struct ExceptionTemplate {
    pub(super) name: String,
    pub(super) fields: String,
}

#[derive(Serialize)]
pub(super) struct EnumMemberTemplate {
    pub(super) name: String,
    pub(super) enum_name: String,
    pub(super) wire_name: String,
}

#[derive(Serialize)]
pub(super) struct EnumTemplate {
    pub(super) name: String,
    pub(super) members: String,
}

#[derive(Serialize)]
pub(super) struct MethodTemplate {
    pub(super) name: String,
    pub(super) request_name: String,
    pub(super) response_name: String,
}

#[derive(Serialize)]
pub(super) struct InterfaceTemplate {
    pub(super) name: String,
    pub(super) methods: String,
}

#[derive(Serialize)]
pub(super) struct OperationTypesTemplate {
    pub(super) request: StructTemplate,
    pub(super) response: StructTemplate,
}

pub(super) fn render_field_block(
    ctx: &GoRenderContext,
    fields: Vec<FieldTemplate>,
) -> IdlcResult<String> {
    let mut rendered = String::new();
    for field in fields {
        rendered.push_str(&ctx.render_template("field.go.j2", &field)?);
    }
    Ok(rendered)
}

pub(super) fn render_method_block(
    ctx: &GoRenderContext,
    methods: &[MethodTemplate],
) -> IdlcResult<String> {
    let mut rendered = String::new();
    for method in methods {
        rendered.push_str(&ctx.render_template("method.go.j2", method)?);
    }
    Ok(rendered)
}

pub(super) fn render_enum_member_block(
    ctx: &GoRenderContext,
    members: Vec<EnumMemberTemplate>,
) -> IdlcResult<String> {
    let mut rendered = String::new();
    for member in members {
        rendered.push_str(&ctx.render_template("enum_member.go.j2", &member)?);
    }
    Ok(rendered)
}
