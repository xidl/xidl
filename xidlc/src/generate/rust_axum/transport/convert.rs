use super::super::{render::RustAxumRenderer, scope::TypeScope};
use super::TransportTypeDef;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::declarator_dims;
use serde::Serialize;
use xidl_parser::hir;

pub(crate) fn encode_expr(
    expr: &str,
    ty: &hir::TypeSpec,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    convert_expr(expr, ty, scope)
}

pub(crate) fn decode_expr(
    expr: &str,
    ty: &hir::TypeSpec,
    scope: TypeScope<'_>,
) -> IdlcResult<String> {
    convert_expr(expr, ty, scope)
}

fn convert_expr(expr: &str, ty: &hir::TypeSpec, scope: TypeScope<'_>) -> IdlcResult<String> {
    let renderer = RustAxumRenderer::new()?;
    OwnedConversion::new(scope, &renderer).value(expr, ty)
}

/// Converts owned values while preserving array and container layers.
pub(super) struct OwnedConversion<'a> {
    scope: TypeScope<'a>,
    renderer: &'a RustAxumRenderer,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Conversion<'a> {
    Map {
        source: &'a str,
        converted: &'a str,
    },
    Box {
        converted: &'a str,
    },
    Sequence {
        source: &'a str,
        converted: &'a str,
        bounded: bool,
    },
    Record {
        source: &'a str,
        converted: &'a str,
        bounded: bool,
    },
}

impl<'a> OwnedConversion<'a> {
    /// Uses the caller's declaration scope and render environment.
    pub(super) fn new(scope: TypeScope<'a>, renderer: &'a RustAxumRenderer) -> Self {
        Self { scope, renderer }
    }

    /// Converts a base value; the destination determines the `Into` target.
    pub(super) fn value(&self, source: &str, ty: &hir::TypeSpec) -> IdlcResult<String> {
        match ty {
            hir::TypeSpec::SequenceType(seq) => {
                let inner = self.value("value", &seq.ty)?;
                if inner == "value" {
                    return Ok(source.to_owned());
                }
                self.render(&Conversion::Sequence {
                    source,
                    converted: &inner,
                    bounded: seq.len.is_some(),
                })
            }
            hir::TypeSpec::MapType(map) => {
                let inner = self.value("value", &map.value)?;
                if inner == "value" {
                    return Ok(source.to_owned());
                }
                self.render(&Conversion::Record {
                    source,
                    converted: &inner,
                    bounded: map.len.is_some(),
                })
            }
            hir::TypeSpec::ScopedName(value) => self.scoped(source, value),
            _ => Ok(source.to_owned()),
        }
    }

    /// Converts each declared fixed-array dimension before the base value.
    pub(super) fn array(
        &self,
        source: &str,
        ty: &hir::TypeSpec,
        dimensions: usize,
    ) -> IdlcResult<String> {
        if dimensions == 0 {
            return self.value(source, ty);
        }
        let converted = self.array("value", ty, dimensions - 1)?;
        self.map(source, &converted)
    }

    /// Maps an optional or fixed-array layer when its value needs conversion.
    pub(super) fn map(&self, source: &str, converted: &str) -> IdlcResult<String> {
        if converted == "value" {
            return Ok(source.to_owned());
        }
        self.render(&Conversion::Map { source, converted })
    }

    /// Restores the indirection required by a recursive public member.
    pub(super) fn boxed(&self, converted: &str) -> IdlcResult<String> {
        self.render(&Conversion::Box { converted })
    }

    fn scoped(&self, source: &str, name: &hir::ScopedName) -> IdlcResult<String> {
        let Some(key) = self.scope.resolve(name) else {
            return Ok(source.to_owned());
        };
        match self.scope.registry.get(key.as_str()) {
            Some(TransportTypeDef::Struct(_))
            | Some(TransportTypeDef::Exception(_))
            | Some(TransportTypeDef::Enum(_)) => Ok(format!("{source}.into()")),
            Some(TransportTypeDef::Typedef { ty, declarator }) => {
                match ty {
                    hir::TypedefType::TypeSpec(inner) => {
                        let mut parent = key.split("::").map(str::to_string).collect::<Vec<_>>();
                        parent.pop();
                        OwnedConversion::new(self.scope.in_declaration(&parent), self.renderer)
                            .array(source, inner, declarator_dims(declarator).len())
                    }
                    hir::TypedefType::ConstrTypeDcl(_) => Err(IdlcError::rpc(format!(
                        "unsupported inline typedef transport for '{key}'"
                    ))),
                }
            }
            Some(TransportTypeDef::Public) | None => Ok(source.to_owned()),
        }
    }

    fn render(&self, conversion: &Conversion<'_>) -> IdlcResult<String> {
        self.renderer
            .render_template("interface/transport_expression.rs.j2", conversion)
    }
}
