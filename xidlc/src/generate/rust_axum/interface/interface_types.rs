use super::RenderEnv;
use xidl_parser::hir;

impl RenderEnv<'_> {
    pub(crate) fn render_param_type(
        &self,
        ty: &hir::TypeSpec,
        optional: bool,
    ) -> crate::error::IdlcResult<String> {
        let inner = self.axum_type(ty)?;
        if optional {
            Ok(format!("Option<{inner}>"))
        } else {
            Ok(inner)
        }
    }

    pub(crate) fn axum_type(&self, ty: &hir::TypeSpec) -> crate::error::IdlcResult<String> {
        self.scope().public_type(ty)
    }

    pub(crate) fn render_scoped_name(&self, value: &hir::ScopedName) -> String {
        self.scope().render(value)
    }
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
        hir::TypeSpec::IntegerType(_)
        | hir::TypeSpec::FloatingPtType
        | hir::TypeSpec::Boolean
        | hir::TypeSpec::CharType
        | hir::TypeSpec::WideCharType => true,
        _ => false,
    }
}

pub(crate) fn cookie_item_is_string(ty: &hir::TypeSpec) -> bool {
    header_item_is_string(ty)
}

pub(crate) fn cookie_item_is_primitive(ty: &hir::TypeSpec) -> bool {
    header_item_is_primitive(ty)
}

impl RenderEnv<'_> {
    pub(crate) fn header_item_ty(&self, ty: &hir::TypeSpec) -> crate::error::IdlcResult<String> {
        match ty {
            hir::TypeSpec::SequenceType(seq) => self.axum_type(&seq.ty),
            _ => self.axum_type(ty),
        }
    }

    pub(crate) fn cookie_item_ty(&self, ty: &hir::TypeSpec) -> crate::error::IdlcResult<String> {
        self.header_item_ty(ty)
    }
}
