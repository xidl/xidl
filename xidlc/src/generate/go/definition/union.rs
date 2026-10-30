//! Typed construction and payload decoding for ordinary tagged unions.

use crate::error::IdlcResult;
use crate::generate::go::GoRenderContext;
use crate::generate::utils::union_wire::UnionWire;
use serde::Serialize;
use xidl_parser::hir;

use super::definition_names::go_export_name;
use super::definition_support::declarator_name;
use super::definition_types::{constr_type_name, type_with_decl};

#[derive(Serialize)]
struct UnionCase {
    name: String,
    ty: String,
    tags: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct UnionTemplate {
    name: String,
    cases: Vec<UnionCase>,
}

impl UnionTemplate {
    /// Builds a typed case codec from the shared discriminator layout.
    pub(super) fn render(
        ctx: &mut GoRenderContext,
        definition: &hir::UnionDef,
        prefix: &[String],
    ) -> IdlcResult<()> {
        let mut nested_scope = prefix.to_vec();
        nested_scope.push(definition.ident.clone());
        for case in &definition.case {
            if let hir::ElementSpecTy::ConstrTypeDcl(constr) = &case.element.ty {
                super::definition_type_render::render_constr_type(ctx, constr, &nested_scope)?;
            }
        }
        let cases = definition
            .case
            .iter()
            .map(|case| UnionCase {
                name: go_export_name(&[], declarator_name(&case.element.value)),
                ty: match &case.element.ty {
                    hir::ElementSpecTy::TypeSpec(ty) => {
                        type_with_decl(ty, &case.element.value, ctx.scope(&nested_scope))
                    }
                    hir::ElementSpecTy::ConstrTypeDcl(ty) => constr_type_name(ty, &nested_scope),
                },
                tags: UnionWire::tags(case),
            })
            .collect();
        ctx.packages
            .require("json", "github.com/xidl/xidl/golang/xidl-go-codec");
        ctx.packages.require("fmt", "fmt");
        ctx.push_template(
            "union.go.j2",
            &Self {
                name: go_export_name(prefix, &definition.ident),
                cases,
            },
        )
    }
}
