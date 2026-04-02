use crate::error::IdlcResult;
use convert_case::{Case, Casing};
use std::fmt::Write;
use xidl_parser::hir;

use super::{GoRenderContext, ParamDirection};

pub(crate) fn render_spec(ctx: &mut GoRenderContext, spec: &hir::Specification) -> IdlcResult<()> {
    for def in &spec.0 {
        render_definition(ctx, def, &[])?;
    }
    Ok(())
}

fn render_definition(
    ctx: &mut GoRenderContext,
    def: &hir::Definition,
    prefix: &[String],
) -> IdlcResult<()> {
    match def {
        hir::Definition::ModuleDcl(module) => {
            let mut next = prefix.to_vec();
            next.push(module.ident.clone());
            for def in &module.definition {
                render_definition(ctx, def, &next)?;
            }
        }
        hir::Definition::TypeDcl(ty) => render_type_dcl(ctx, ty, prefix)?,
        hir::Definition::ConstDcl(const_dcl) => render_const(ctx, const_dcl, prefix)?,
        hir::Definition::ExceptDcl(except) => render_exception(ctx, except, prefix)?,
        hir::Definition::InterfaceDcl(interface) => {
            if ctx
                .properties
                .get("enable_interfaces")
                .and_then(|value| value.as_bool())
                .unwrap_or(true)
            {
                render_interface(ctx, interface, prefix)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_const(
    ctx: &mut GoRenderContext,
    def: &hir::ConstDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    let ty = go_const_type(&def.ty);
    let value = render_const_expr(&def.value)?;
    writeln!(&mut ctx.body, "const {name} {ty} = {value}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_exception(
    ctx: &mut GoRenderContext,
    def: &hir::ExceptDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
    for member in &def.member {
        for decl in &member.ident {
            let field = go_field_name(declarator_name(decl));
            let mut ty = type_with_decl(&member.ty, decl);
            if member.is_optional() {
                ty = pointer_type(&ty);
            }
            writeln!(
                &mut ctx.body,
                "\t{field} {ty} `json:\"{}\"`",
                declarator_name(decl)
            )
            .unwrap();
        }
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    writeln!(&mut ctx.body, "func (e *{name}) Error() string {{").unwrap();
    writeln!(&mut ctx.body, "\treturn \"{name}\"").unwrap();
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_type_dcl(
    ctx: &mut GoRenderContext,
    def: &hir::TypeDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match &def.decl {
        hir::TypeDclInner::ConstrTypeDcl(constr) => render_constr_type(ctx, constr, prefix),
        hir::TypeDclInner::TypedefDcl(typedef) => render_typedef(ctx, typedef, prefix),
        hir::TypeDclInner::NativeDcl(native) => {
            let name = go_export_name(prefix, &native.decl.0);
            writeln!(&mut ctx.body, "type {name} any").unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
    }
}

fn render_typedef(
    ctx: &mut GoRenderContext,
    def: &hir::TypedefDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match &def.ty {
        hir::TypedefType::TypeSpec(ty) => {
            for decl in &def.decl {
                let name = go_field_name(declarator_name(decl));
                let base = type_with_decl(ty, decl);
                writeln!(
                    &mut ctx.body,
                    "type {} = {}",
                    go_export_name(prefix, &name),
                    base
                )
                .unwrap();
                writeln!(&mut ctx.body).unwrap();
            }
        }
        hir::TypedefType::ConstrTypeDcl(constr) => {
            render_constr_type(ctx, constr, prefix)?;
            let base = constr_type_name(constr, prefix);
            for decl in &def.decl {
                writeln!(
                    &mut ctx.body,
                    "type {} = {}",
                    go_export_name(prefix, declarator_name(decl)),
                    base
                )
                .unwrap();
                writeln!(&mut ctx.body).unwrap();
            }
        }
    }
    Ok(())
}

fn render_constr_type(
    ctx: &mut GoRenderContext,
    constr: &hir::ConstrTypeDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => render_struct(ctx, def, prefix),
        hir::ConstrTypeDcl::EnumDcl(def) => render_enum(ctx, def, prefix),
        hir::ConstrTypeDcl::UnionDef(def) => {
            let name = go_export_name(prefix, &def.ident);
            writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
            writeln!(&mut ctx.body, "\tTag string `json:\"tag\"`").unwrap();
            writeln!(&mut ctx.body, "\tValue any `json:\"value\"`").unwrap();
            writeln!(&mut ctx.body, "}}").unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
        hir::ConstrTypeDcl::BitsetDcl(def) => {
            let name = go_export_name(prefix, &def.ident);
            writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
            writeln!(&mut ctx.body, "\tBits uint64 `json:\"bits\"`").unwrap();
            writeln!(&mut ctx.body, "}}").unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            let name = go_export_name(prefix, &def.ident);
            writeln!(&mut ctx.body, "type {name} uint64").unwrap();
            writeln!(&mut ctx.body).unwrap();
            Ok(())
        }
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => Ok(()),
    }
}

fn render_struct(
    ctx: &mut GoRenderContext,
    def: &hir::StructDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    writeln!(&mut ctx.body, "type {name} struct {{").unwrap();
    for member in &def.member {
        for decl in &member.ident {
            let field = go_field_name(declarator_name(decl));
            let mut ty = type_with_decl(&member.ty, decl);
            if member.is_optional() {
                ty = pointer_type(&ty);
            }
            writeln!(
                &mut ctx.body,
                "\t{field} {ty} `json:\"{}\"`",
                declarator_name(decl)
            )
            .unwrap();
        }
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_enum(ctx: &mut GoRenderContext, def: &hir::EnumDcl, prefix: &[String]) -> IdlcResult<()> {
    let name = go_export_name(prefix, &def.ident);
    writeln!(&mut ctx.body, "type {name} string").unwrap();
    writeln!(&mut ctx.body).unwrap();
    writeln!(&mut ctx.body, "const (").unwrap();
    for member in &def.member {
        let value_name = format!("{}{}", name, member.ident.to_case(Case::Pascal));
        writeln!(
            &mut ctx.body,
            "\t{value_name} {name} = \"{}\"",
            member.ident
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, ")").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_interface(
    ctx: &mut GoRenderContext,
    def: &hir::InterfaceDcl,
    prefix: &[String],
) -> IdlcResult<()> {
    let hir::InterfaceDclInner::InterfaceDef(def) = &def.decl else {
        return Ok(());
    };
    ctx.state.uses_context = true;
    let interface_name = go_export_name(prefix, &def.header.ident);
    let mut methods = Vec::new();
    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    let (request_name, response_name) =
                        render_operation_types(ctx, &interface_name, op)?;
                    methods.push((go_field_name(&op.ident), request_name, response_name));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(render_attr_types(ctx, &interface_name, attr)?);
                }
                hir::Export::TypeDcl(type_dcl) => render_type_dcl(ctx, type_dcl, prefix)?,
                hir::Export::ConstDcl(const_dcl) => render_const(ctx, const_dcl, prefix)?,
                hir::Export::ExceptDcl(except) => render_exception(ctx, except, prefix)?,
            }
        }
    }
    writeln!(&mut ctx.body, "type {interface_name} interface {{").unwrap();
    for (method_name, request_name, response_name) in methods {
        writeln!(
            &mut ctx.body,
            "\t{method_name}(ctx context.Context, req *{request_name}) (*{response_name}, error)"
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok(())
}

fn render_operation_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    op: &hir::OpDcl,
) -> IdlcResult<(String, String)> {
    let request_name = format!(
        "{}{}Request",
        interface_name,
        op.ident.to_case(Case::Pascal)
    );
    let response_name = format!(
        "{}{}Response",
        interface_name,
        op.ident.to_case(Case::Pascal)
    );
    writeln!(&mut ctx.body, "type {request_name} struct {{").unwrap();
    for param in operation_params(op)
        .iter()
        .filter(|param| !is_out_param(param.attr.as_ref()))
    {
        let field = go_field_name(&param.declarator.0);
        writeln!(
            &mut ctx.body,
            "\t{field} {} `json:\"{}\"`",
            go_type(&param.ty),
            param.declarator.0
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();

    writeln!(&mut ctx.body, "type {response_name} struct {{").unwrap();
    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        writeln!(&mut ctx.body, "\tReturn {} `json:\"return\"`", go_type(ty)).unwrap();
    }
    for param in operation_params(op).iter().filter(|param| {
        matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        )
    }) {
        let field = go_field_name(&param.declarator.0);
        writeln!(
            &mut ctx.body,
            "\t{field} {} `json:\"{}\"`",
            go_type(&param.ty),
            param.declarator.0
        )
        .unwrap();
    }
    writeln!(&mut ctx.body, "}}").unwrap();
    writeln!(&mut ctx.body).unwrap();
    Ok((request_name, response_name))
}

fn render_attr_types(
    ctx: &mut GoRenderContext,
    interface_name: &str,
    attr: &hir::AttrDcl,
) -> IdlcResult<Vec<(String, String, String)>> {
    let mut out = Vec::new();
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => {
            if let hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) = &spec.declarator {
                let raw = decl.0.clone();
                let request_name = format!(
                    "{}GetAttribute{}Request",
                    interface_name,
                    raw.to_case(Case::Pascal)
                );
                let response_name = format!(
                    "{}GetAttribute{}Response",
                    interface_name,
                    raw.to_case(Case::Pascal)
                );
                writeln!(&mut ctx.body, "type {request_name} struct {{}}").unwrap();
                writeln!(&mut ctx.body).unwrap();
                writeln!(&mut ctx.body, "type {response_name} struct {{").unwrap();
                writeln!(
                    &mut ctx.body,
                    "\tReturn {} `json:\"return\"`",
                    go_type(&spec.ty)
                )
                .unwrap();
                writeln!(&mut ctx.body, "}}").unwrap();
                writeln!(&mut ctx.body).unwrap();
                out.push((
                    format!("GetAttribute{}", raw.to_case(Case::Pascal)),
                    request_name,
                    response_name,
                ));
            }
        }
        hir::AttrDclInner::AttrSpec(spec) => {
            if let hir::AttrDeclarator::SimpleDeclarator(values) = &spec.declarator {
                for decl in values {
                    let raw = decl.0.clone();
                    let get_request_name = format!(
                        "{}GetAttribute{}Request",
                        interface_name,
                        raw.to_case(Case::Pascal)
                    );
                    let get_response_name = format!(
                        "{}GetAttribute{}Response",
                        interface_name,
                        raw.to_case(Case::Pascal)
                    );
                    writeln!(&mut ctx.body, "type {get_request_name} struct {{}}").unwrap();
                    writeln!(&mut ctx.body).unwrap();
                    writeln!(&mut ctx.body, "type {get_response_name} struct {{").unwrap();
                    writeln!(
                        &mut ctx.body,
                        "\tReturn {} `json:\"return\"`",
                        go_type(&spec.ty)
                    )
                    .unwrap();
                    writeln!(&mut ctx.body, "}}").unwrap();
                    writeln!(&mut ctx.body).unwrap();
                    out.push((
                        format!("GetAttribute{}", raw.to_case(Case::Pascal)),
                        get_request_name,
                        get_response_name,
                    ));

                    let set_request_name = format!(
                        "{}SetAttribute{}Request",
                        interface_name,
                        raw.to_case(Case::Pascal)
                    );
                    let set_response_name = format!(
                        "{}SetAttribute{}Response",
                        interface_name,
                        raw.to_case(Case::Pascal)
                    );
                    writeln!(&mut ctx.body, "type {set_request_name} struct {{").unwrap();
                    writeln!(
                        &mut ctx.body,
                        "\tValue {} `json:\"value\"`",
                        go_type(&spec.ty)
                    )
                    .unwrap();
                    writeln!(&mut ctx.body, "}}").unwrap();
                    writeln!(&mut ctx.body).unwrap();
                    writeln!(&mut ctx.body, "type {set_response_name} struct {{}}").unwrap();
                    writeln!(&mut ctx.body).unwrap();
                    out.push((
                        format!("SetAttribute{}", raw.to_case(Case::Pascal)),
                        set_request_name,
                        set_response_name,
                    ));
                }
            }
        }
    }
    Ok(out)
}

pub(crate) fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(crate) fn is_out_param(attr: Option<&hir::ParamAttribute>) -> bool {
    matches!(param_direction(attr), ParamDirection::Out)
}

pub(crate) fn operation_params(op: &hir::OpDcl) -> &[hir::ParamDcl] {
    op.parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[])
}

pub(crate) fn constr_type_name(constr: &hir::ConstrTypeDcl, prefix: &[String]) -> String {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::StructForwardDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::EnumDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::UnionDef(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::BitsetDcl(def) => go_export_name(prefix, &def.ident),
        hir::ConstrTypeDcl::BitmaskDcl(def) => go_export_name(prefix, &def.ident),
    }
}

pub(crate) fn declarator_name(decl: &hir::Declarator) -> &str {
    match decl {
        hir::Declarator::SimpleDeclarator(value) => &value.0,
        hir::Declarator::ArrayDeclarator(value) => &value.ident,
    }
}

pub(crate) fn type_with_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> String {
    let base = go_type(ty);
    match decl {
        hir::Declarator::SimpleDeclarator(_) => base,
        hir::Declarator::ArrayDeclarator(value) => {
            let mut out = base;
            for len in value.len.iter().rev() {
                out = format!(
                    "[{}]{}",
                    render_const_expr(&len.0).unwrap_or_else(|_| "0".to_string()),
                    out
                );
            }
            out
        }
    }
}

pub(crate) fn render_const_expr(expr: &hir::ConstExpr) -> IdlcResult<String> {
    Ok(crate::generate::render_const_expr(
        expr,
        &|value| go_scoped_name(value),
        &|literal| go_literal(literal),
    ))
}

pub(crate) fn go_scoped_name(value: &hir::ScopedName) -> String {
    value
        .name
        .iter()
        .map(|part| part.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn go_literal(value: &hir::Literal) -> String {
    match value {
        hir::Literal::IntegerLiteral(lit) => match lit {
            hir::IntegerLiteral::BinNumber(value)
            | hir::IntegerLiteral::OctNumber(value)
            | hir::IntegerLiteral::DecNumber(value)
            | hir::IntegerLiteral::HexNumber(value) => value.clone(),
        },
        hir::Literal::FloatingPtLiteral(lit) => {
            let sign = lit
                .sign
                .as_ref()
                .map(|value| value.0.as_str())
                .unwrap_or("");
            format!("{}{}.{}", sign, lit.integer.0, lit.fraction.0)
        }
        hir::Literal::CharLiteral(value) => value.clone(),
        hir::Literal::WideCharacterLiteral(value) => {
            value.strip_prefix('L').unwrap_or(value).into()
        }
        hir::Literal::StringLiteral(value) => value.clone(),
        hir::Literal::WideStringLiteral(value) => value.strip_prefix('L').unwrap_or(value).into(),
        hir::Literal::BooleanLiteral(value) => value.to_ascii_lowercase(),
    }
}

pub(crate) fn go_const_type(ty: &hir::ConstType) -> String {
    match ty {
        hir::ConstType::IntegerType(value) => go_integer_type(value),
        hir::ConstType::FloatingPtType => "float64".to_string(),
        hir::ConstType::FixedPtConstType => "float64".to_string(),
        hir::ConstType::CharType => "rune".to_string(),
        hir::ConstType::WideCharType => "rune".to_string(),
        hir::ConstType::BooleanType => "bool".to_string(),
        hir::ConstType::OctetType => "byte".to_string(),
        hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_) => "string".to_string(),
        hir::ConstType::ScopedName(value) => go_scoped_name(value),
        hir::ConstType::SequenceType(_) => "any".to_string(),
    }
}

pub(crate) fn go_type(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => go_integer_type(value),
            hir::SimpleTypeSpec::FloatingPtType => "float64".to_string(),
            hir::SimpleTypeSpec::CharType => "rune".to_string(),
            hir::SimpleTypeSpec::WideCharType => "rune".to_string(),
            hir::SimpleTypeSpec::Boolean => "bool".to_string(),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => "any".to_string(),
            hir::SimpleTypeSpec::ScopedName(value) => go_scoped_name(value),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => format!("[]{}", go_type(&seq.ty)),
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                "string".to_string()
            }
            hir::TemplateTypeSpec::FixedPtType(_) => "float64".to_string(),
            hir::TemplateTypeSpec::MapType(map) => {
                format!("map[{}]{}", go_type(&map.key), go_type(&map.value))
            }
            hir::TemplateTypeSpec::TemplateType(value) => format!(
                "{}[{}]",
                value.ident.to_case(Case::Pascal),
                value
                    .args
                    .iter()
                    .map(go_type)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        },
    }
}

pub(crate) fn go_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "int8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::U8 => "uint8".to_string(),
        hir::IntegerType::U16 => "uint16".to_string(),
        hir::IntegerType::U32 => "uint32".to_string(),
        hir::IntegerType::U64 => "uint64".to_string(),
        hir::IntegerType::I8 => "int8".to_string(),
        hir::IntegerType::I16 => "int16".to_string(),
        hir::IntegerType::I32 => "int32".to_string(),
        hir::IntegerType::I64 => "int64".to_string(),
    }
}

pub(crate) fn go_export_name(prefix: &[String], value: &str) -> String {
    let mut parts = prefix
        .iter()
        .map(|part| part.to_case(Case::Pascal))
        .collect::<Vec<_>>();
    parts.push(value.to_case(Case::Pascal));
    parts.join("")
}

pub(crate) fn go_field_name(value: &str) -> String {
    let ident = value.to_case(Case::Pascal);
    if go_keyword(&ident) {
        format!("{ident}_")
    } else {
        ident
    }
}

pub(crate) fn pointer_type(ty: &str) -> String {
    if ty.starts_with('*') {
        ty.to_string()
    } else {
        format!("*{ty}")
    }
}

pub(crate) fn go_keyword(value: &str) -> bool {
    matches!(
        value,
        "Break"
            | "Default"
            | "Func"
            | "Interface"
            | "Select"
            | "Case"
            | "Defer"
            | "Go"
            | "Map"
            | "Struct"
            | "Chan"
            | "Else"
            | "Goto"
            | "Package"
            | "Switch"
            | "Const"
            | "Fallthrough"
            | "If"
            | "Range"
            | "Type"
            | "Continue"
            | "For"
            | "Import"
            | "Return"
            | "Var"
    )
}
