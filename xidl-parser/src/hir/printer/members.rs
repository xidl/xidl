use super::super::*;
use super::IdlPrinter;

impl IdlPrinter {
    pub(super) fn member(&mut self, member: &Member, implicit_id: Option<usize>) {
        self.annotations(&member.annotations);
        if let Some(id) = member.field_id {
            if implicit_id.is_some_and(|implicit| implicit != id as usize)
                && annotation_id_value(&member.annotations).is_none()
            {
                self.line(format!("@id({id})"));
            }
        }
        let default = member
            .default
            .as_ref()
            .map(|value| format!(" default {}", Self::render_expr(&value.0)))
            .unwrap_or_default();
        self.line(format!(
            "{} {}{};",
            Self::render_type(&member.ty),
            member
                .ident
                .iter()
                .map(Self::render_declarator)
                .collect::<Vec<_>>()
                .join(", "),
            default
        ));
    }

    pub(super) fn const_dcl(&mut self, value: &ConstDcl) {
        self.annotations(&value.annotations);
        self.line(format!(
            "const {} {} = {};",
            Self::render_const_type(&value.ty),
            value.ident,
            Self::render_expr(&value.value)
        ));
    }

    pub(super) fn exception(&mut self, value: &ExceptDcl) {
        self.annotations(&value.annotations);
        self.line(format!("exception {} {{", value.ident));
        self.indent += 1;
        for member in &value.member {
            self.member(member, None);
        }
        self.indent -= 1;
        self.line("};");
    }
}
