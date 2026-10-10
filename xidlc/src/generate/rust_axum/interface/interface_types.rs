use super::RenderEnv;
use crate::generate::rust::util::rust_ident;
use xidl_parser::hir;

impl RenderEnv<'_> {
    pub(crate) fn render_param_type(&self, ty: &hir::TypeSpec, optional: bool) -> String {
        let inner = self.axum_type(ty);
        if optional {
            format!("Option<{inner}>")
        } else {
            inner
        }
    }

    pub(crate) fn axum_type(&self, ty: &hir::TypeSpec) -> String {
        match ty {
            hir::TypeSpec::IntegerType(value) => rust_integer_type(value),
            hir::TypeSpec::FloatingPtType => "f64".to_string(),
            hir::TypeSpec::CharType | hir::TypeSpec::WideCharType => "char".to_string(),
            hir::TypeSpec::Boolean => "bool".to_string(),
            hir::TypeSpec::AnyType | hir::TypeSpec::ObjectType | hir::TypeSpec::ValueBaseType => {
                "xidl_rust_axum::serde_json::Value".to_string()
            }
            hir::TypeSpec::ScopedName(value) => self.render_scoped_name(value),
            hir::TypeSpec::SequenceType(seq) => format!("Vec<{}>", self.axum_type(&seq.ty)),
            hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => "String".to_string(),
            hir::TypeSpec::FixedPtType(_) => "f64".to_string(),
            hir::TypeSpec::MapType(map) => format!(
                "::std::collections::BTreeMap<{}, {}>",
                self.axum_type(&map.key),
                self.axum_type(&map.value)
            ),
            hir::TypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                rust_ident(&value.ident),
                value
                    .args
                    .iter()
                    .map(|ty| self.axum_type(ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub(crate) fn render_scoped_name(&self, value: &hir::ScopedName) -> String {
        self.scope().render(value)
    }
}

fn rust_integer_type(value: &hir::IntegerType) -> String {
    match value {
        hir::IntegerType::Char => "i8".to_string(),
        hir::IntegerType::UChar | hir::IntegerType::Octet | hir::IntegerType::U8 => {
            "u8".to_string()
        }
        hir::IntegerType::U16 => "u16".to_string(),
        hir::IntegerType::U32 => "u32".to_string(),
        hir::IntegerType::U64 => "u64".to_string(),
        hir::IntegerType::I8 => "i8".to_string(),
        hir::IntegerType::I16 => "i16".to_string(),
        hir::IntegerType::I32 => "i32".to_string(),
        hir::IntegerType::I64 => "i64".to_string(),
    }
}

pub(crate) fn header_is_multi(ty: &hir::TypeSpec) -> bool {
    matches!(ty, hir::TypeSpec::SequenceType(_))
}

pub(crate) fn header_item_is_string(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_string(&seq.ty),
        hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => true,
        _ => false,
    }
}

pub(crate) fn header_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    match ty {
        hir::TypeSpec::SequenceType(seq) => header_item_is_primitive(&seq.ty),
        hir::TypeSpec::IntegerType(_) | hir::TypeSpec::FloatingPtType | hir::TypeSpec::Boolean => {
            true
        }
        _ => false,
    }
}

pub(crate) fn cookie_is_multi(ty: &hir::TypeSpec) -> bool {
    header_is_multi(ty)
}

pub(crate) fn cookie_item_is_string(ty: &hir::TypeSpec) -> bool {
    header_item_is_string(ty)
}

pub(crate) fn cookie_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    header_item_is_primitive(ty)
}

impl RenderEnv<'_> {
    pub(crate) fn header_item_ty(&self, ty: &hir::TypeSpec) -> String {
        match ty {
            hir::TypeSpec::SequenceType(seq) => self.axum_type(&seq.ty),
            _ => self.axum_type(ty),
        }
    }

    pub(crate) fn cookie_item_ty(&self, ty: &hir::TypeSpec) -> String {
        self.header_item_ty(ty)
    }
}
