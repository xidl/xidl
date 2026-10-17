//! Canonical rendering of the semantic HIR back to parseable IDL.

use super::*;

/// Prints semantic HIR as canonical, parseable IDL.
///
/// Documentation annotations are presentation metadata and are omitted so
/// comment whitespace and placement cannot change a contract diff.
pub struct IdlPrinter {
    output: String,
    indent: usize,
}

impl IdlPrinter {
    /// Renders a specification without applying source formatting.
    pub fn render(spec: &Specification) -> String {
        let mut printer = Self {
            output: String::new(),
            indent: 0,
        };
        for definition in &spec.0 {
            printer.definition(definition);
        }
        printer.output
    }

    fn line(&mut self, text: impl AsRef<str>) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(text.as_ref());
        self.output.push('\n');
    }

    fn annotations(&mut self, annotations: &[Annotation]) {
        for annotation in annotations {
            if Self::is_doc_annotation(annotation) {
                continue;
            }
            self.line(Self::render_annotation(annotation));
        }
    }

    fn definition(&mut self, definition: &Definition) {
        match definition {
            Definition::ModuleDcl(value) => self.module(value),
            Definition::Pragma(value) => self.pragma(value),
            Definition::ConstrTypeDcl(value) => self.constr(value),
            Definition::TypeDcl(value) => self.type_dcl(value),
            Definition::ConstDcl(value) => self.const_dcl(value),
            Definition::ExceptDcl(value) => self.exception(value),
            Definition::InterfaceDcl(value) => self.interface(value),
        }
    }

    fn module(&mut self, module: &ModuleDcl) {
        self.annotations(&module.annotations);
        self.line(format!("module {} {{", module.ident));
        self.indent += 1;
        for definition in &module.definition {
            self.definition(definition);
        }
        self.indent -= 1;
        self.line("};");
    }

    fn pragma(&mut self, pragma: &Pragma) {
        let text = match pragma {
            Pragma::Custom(value) => match &value.argument {
                Some(argument) => format!("{} {}", value.directive, argument),
                None => value.directive.clone(),
            },
            Pragma::XidlcPackage(value) => format!("#pragma xidlc package {}", Self::quote(value)),
            Pragma::XidlcOpenApiVersion(value) => {
                format!("#pragma xidlc version {}", Self::quote(value))
            }
            Pragma::XidlcOpenApiService {
                base_url,
                description,
            } => match description {
                Some(description) => format!(
                    "#pragma xidlc service {} {}",
                    Self::quote(base_url),
                    Self::quote(description)
                ),
                None => format!("#pragma xidlc service {}", Self::quote(base_url)),
            },
        };
        self.line(text);
    }

    fn type_dcl(&mut self, declaration: &TypeDcl) {
        match declaration {
            TypeDcl::ConstrTypeDcl(value) => self.constr(value),
            TypeDcl::TypedefDcl(value) => {
                self.annotations(&value.annotations);
                let declarators = value
                    .decl
                    .iter()
                    .map(Self::render_declarator)
                    .collect::<Vec<_>>()
                    .join(", ");
                match &value.ty {
                    TypedefType::TypeSpec(ty) => {
                        self.line(format!(
                            "typedef {} {};",
                            Self::render_type(ty),
                            declarators
                        ));
                    }
                    TypedefType::ConstrTypeDcl(constr) => {
                        self.typedef_constr(constr, &declarators);
                    }
                }
            }
            TypeDcl::NativeDcl(value) => {
                self.annotations(&value.annotations);
                self.line(format!("native {};", value.decl.0));
            }
        }
    }

    fn constr(&mut self, declaration: &ConstrTypeDcl) {
        self.constructed(declaration, "", ";");
    }

    fn interface(&mut self, value: &InterfaceDcl) {
        self.annotations(&value.annotations);
        match &value.decl {
            InterfaceDclInner::InterfaceForwardDcl(value) => {
                self.line(format!("interface {};", value.ident));
            }
            InterfaceDclInner::InterfaceDef(value) => {
                let parent = value
                    .header
                    .parent
                    .as_ref()
                    .map(|parent| {
                        format!(
                            " : {}",
                            parent
                                .0
                                .iter()
                                .map(|name| Self::render_scoped(&name.0))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .unwrap_or_default();
                self.line(format!("interface {}{} {{", value.header.ident, parent));
                self.indent += 1;
                if let Some(body) = &value.interface_body {
                    for export in &body.0 {
                        self.export(export);
                    }
                }
                self.indent -= 1;
                self.line("};");
            }
        }
    }

    fn export(&mut self, export: &Export) {
        match export {
            Export::OpDcl(value) => self.operation(value),
            Export::AttrDcl(value) => self.attribute(value),
            Export::TypeDcl(value) => self.type_dcl(value),
            Export::ConstDcl(value) => self.const_dcl(value),
            Export::ExceptDcl(value) => self.exception(value),
        }
    }

    fn operation(&mut self, value: &OpDcl) {
        self.annotations(&value.annotations);
        let return_type = match &value.ty {
            OpTypeSpec::Void => "void".to_string(),
            OpTypeSpec::TypeSpec(value) => Self::render_type(value),
        };
        let params = value
            .parameter
            .as_ref()
            .map(|params| {
                params
                    .0
                    .iter()
                    .map(|param| {
                        let attr = param
                            .attr
                            .as_ref()
                            .map(|attr| format!("{} ", attr.0))
                            .unwrap_or_default();
                        format!(
                            "{}{} {} {}",
                            Self::render_annotations_inline(&param.annotations),
                            attr,
                            Self::render_type(&param.ty),
                            param.declarator.0
                        )
                        .trim()
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let raises = value
            .raises
            .as_ref()
            .map(|raises| {
                format!(
                    " raises({})",
                    raises
                        .0
                        .iter()
                        .map(Self::render_scoped)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .unwrap_or_default();
        self.line(format!(
            "{} {}({}){};",
            return_type, value.ident, params, raises
        ));
    }

    fn attribute(&mut self, value: &AttrDcl) {
        self.annotations(&value.annotations);
        match &value.decl {
            AttrDclInner::ReadonlyAttrSpec(value) => {
                let suffix = Self::render_readonly_declarator(&value.declarator);
                self.line(format!(
                    "readonly attribute {}{};",
                    Self::render_type(&value.ty),
                    suffix
                ));
            }
            AttrDclInner::AttrSpec(value) => {
                let suffix = Self::render_attr_declarator(&value.declarator);
                self.line(format!(
                    "attribute {}{};",
                    Self::render_type(&value.ty),
                    suffix
                ));
            }
        }
    }
}

mod annotation;
mod constr;
mod members;
mod render;

#[cfg(test)]
mod tests;
