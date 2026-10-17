use super::super::*;
use super::IdlPrinter;
use std::collections::HashMap;

impl IdlPrinter {
    pub(super) fn typedef_constr(&mut self, value: &ConstrTypeDcl, declarators: &str) {
        self.constructed(value, "typedef ", &format!(" {declarators};"));
    }

    pub(super) fn inline_constr(&mut self, value: &ConstrTypeDcl, declarator: &str) {
        self.constructed(value, "", &format!(" {declarator};"));
    }

    pub(super) fn constructed(&mut self, value: &ConstrTypeDcl, prefix: &str, suffix: &str) {
        self.annotations(Self::value_annotations(value));
        match value {
            ConstrTypeDcl::StructForwardDcl(value) => {
                self.line(format!("{prefix}struct {}{suffix}", value.ident));
                return;
            }
            ConstrTypeDcl::UnionForwardDcl(value) => {
                self.line(format!("{prefix}union {}{suffix}", value.ident));
                return;
            }
            ConstrTypeDcl::StructDcl(value) => {
                self.line(format!(
                    "{prefix}struct {}{} {{",
                    value.ident,
                    Self::parents(&value.parent)
                ));
                self.indent += 1;
                for (index, member) in value.member.iter().enumerate() {
                    self.member(member, Some(index + 1));
                }
            }
            ConstrTypeDcl::EnumDcl(value) => {
                self.line(format!("{prefix}enum {} {{", value.ident));
                self.indent += 1;
                for member in &value.member {
                    self.annotations(&member.annotations);
                    self.line(format!("{},", member.ident));
                }
            }
            ConstrTypeDcl::UnionDef(value) => {
                self.line(format!(
                    "{prefix}union {} switch ({}) {{",
                    value.ident,
                    Self::render_switch_type(&value.switch_type_spec)
                ));
                self.indent += 1;
                let mut implicit_ids = HashMap::new();
                let mut next_id = 1u32;
                for case in &value.case {
                    self.case_labels(case);
                    self.annotations(&case.element.annotations);
                    let name = Self::render_declarator(&case.element.value);
                    let explicit_id = annotation_id_value(&case.element.annotations);
                    let implicit_id = *implicit_ids.entry(name).or_insert_with(|| {
                        let id = explicit_id.unwrap_or(next_id);
                        if explicit_id.is_none() {
                            next_id += 1;
                        }
                        id
                    });
                    // Projection can remove earlier union members, changing
                    // the parser's implicit IDs. Preserve only changed IDs.
                    if let Some(id) = case.element.field_id {
                        if id != implicit_id && explicit_id.is_none() {
                            self.line(format!("@id({id})"));
                        }
                    }
                    self.inline_element(&case.element);
                }
            }
            ConstrTypeDcl::BitsetDcl(value) => {
                let parent = value.parent.as_slice();
                self.line(format!(
                    "{prefix}bitset {}{} {{",
                    value.ident,
                    Self::parents(parent)
                ));
                self.indent += 1;
                for field in &value.field {
                    let ty = field
                        .ty
                        .as_ref()
                        .map(Self::render_bitfield_type)
                        .map(|ty| format!(", {ty}"))
                        .unwrap_or_default();
                    self.line(format!(
                        "bitfield<{}{}> {};",
                        Self::render_expr(&field.pos.0),
                        ty,
                        field.ident
                    ));
                }
            }
            ConstrTypeDcl::BitmaskDcl(value) => {
                self.line(format!("{prefix}bitmask {} {{", value.ident));
                self.indent += 1;
                for (index, bit) in value.value.iter().enumerate() {
                    self.annotations(&bit.annotations);
                    let explicit = bit
                        .annotations
                        .iter()
                        .any(|annotation| matches!(annotation, Annotation::Position { .. }));
                    if bit.position != index && !explicit {
                        self.line(format!("@position({})", bit.position));
                    }
                    self.line(format!("{},", bit.ident));
                }
            }
        }
        self.indent -= 1;
        self.line(format!("}}{suffix}"));
    }

    fn parents(parents: &[ScopedName]) -> String {
        if parents.is_empty() {
            return String::new();
        }
        format!(
            " : {}",
            parents
                .iter()
                .map(Self::render_scoped)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub(super) fn inline_element(&mut self, element: &ElementSpec) {
        match &element.ty {
            ElementSpecTy::TypeSpec(ty) => self.line(format!(
                "{} {};",
                Self::render_type(ty),
                Self::render_declarator(&element.value)
            )),
            ElementSpecTy::ConstrTypeDcl(constr) => {
                self.inline_constr(constr, &Self::render_declarator(&element.value));
            }
        }
    }

    pub(super) fn case_labels(&mut self, case: &Case) {
        for label in &case.label {
            let label = match label {
                CaseLabel::Value(value) => format!("case {}:", Self::render_expr(value)),
                CaseLabel::Default => "default:".to_string(),
            };
            self.line(label);
        }
    }
}
