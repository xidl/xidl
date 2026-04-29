use crate::generate::http_hir::{HttpParam, HttpParamKind};
use crate::generate::rust_axum::interface::types::axum_type;
use xidl_parser::hir;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ParamDirection {
    In,
    Out,
    InOut,
}

pub fn find_http_param<'a>(params: &'a [HttpParam], name: &str) -> Option<&'a HttpParam> {
    params.iter().find(|param| param.name == name)
}

pub fn http_param_kind(source: HttpParamKind) -> ParamSource {
    match source {
        HttpParamKind::Path => ParamSource::Path,
        HttpParamKind::Query => ParamSource::Query,
        HttpParamKind::Header => ParamSource::Header,
        HttpParamKind::Cookie => ParamSource::Cookie,
        HttpParamKind::Body => ParamSource::Body,
    }
}

pub fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub fn header_is_multi(ty: &hir::TypeSpec) -> bool {
    matches!(ty, hir::TypeSpec::SequenceType(_))
}

pub fn header_item_ty(ty: &hir::TypeSpec) -> String {
    match ty {
        hir::TypeSpec::SequenceType(seq) => axum_type(&seq.ty),
        _ => axum_type(ty),
    }
}

pub fn header_item_is_string(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_string(&seq.ty),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => true,
        _ => false,
    }
}

pub fn header_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_primitive(&seq.ty),
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FloatingPtType | hir::TypeSpec::Boolean => {
            true
        }
        _ => false,
    }
}

pub fn cookie_is_multi(ty: &hir::TypeSpec) -> bool {
    header_is_multi(ty)
}

pub fn cookie_item_ty(ty: &hir::TypeSpec) -> String {
    header_item_ty(ty)
}

pub fn cookie_item_is_string(ty: &hir::TypeSpec) -> bool {
    header_item_is_string(ty)
}

pub fn cookie_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    header_item_is_primitive(ty)
}

pub fn param_source_code(source: ParamSource) -> String {
    match source {
        ParamSource::Query => "ParamSource::Query".to_string(),
        ParamSource::Body => "ParamSource::Body".to_string(),
        ParamSource::Path => "ParamSource::Path".to_string(),
        ParamSource::Header => "ParamSource::Header".to_string(),
        ParamSource::Cookie => "ParamSource::Cookie".to_string(),
    }
}
