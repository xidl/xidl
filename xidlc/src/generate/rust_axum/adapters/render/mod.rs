//! Assemble Rust adapter contexts without parser-internal lifecycle structures.

mod conversions;
mod endpoints;
mod operations;
mod unions;
mod wire;

use super::context::*;
use super::layout::{Declaration, Layout};
use super::names::Names;
use crate::error::IdlcResult;
use crate::generate::adapter::{AdapterPlan, Member, TypeConversion};
use crate::generate::rust::util::rust_ident;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;
use xidl_parser::rest_hir::RestHirDocument;

pub(super) struct AdapterRenderer<'a> {
    pub http: &'a RestHirDocument,
    pub plan: &'a AdapterPlan,
    pub layout: Layout<'a>,
}

impl<'a> AdapterRenderer<'a> {
    pub(super) fn new(http: &'a RestHirDocument, plan: &'a AdapterPlan) -> Self {
        Self {
            http,
            plan,
            layout: Layout::new(&http.spec),
        }
    }

    pub(super) fn context(&self) -> IdlcResult<FileContext> {
        let mut file = FileContext::default();
        for version in &self.plan.versions {
            file.versions.push(Version {
                id: version.version.id.to_case(Case::Pascal),
                module: rust_ident(&version.version.id),
                raw_id: version.version.id.clone(),
                value: version.version.value.clone(),
            });
            let names = Names {
                version: &version.version.id,
                step_from: &version.version.id,
                upward: false,
                types: &version.surface.types,
                layout: &self.layout,
            };
            for operation in version.surface.operations.values() {
                file.requests.push(self.request(operation, &names)?);
            }
        }
        for step in &self.plan.steps {
            let old = Names {
                version: &step.from.id,
                step_from: &step.from.id,
                upward: false,
                types: &step.source_types,
                layout: &self.layout,
            };
            let new = Names {
                version: &step.to.id,
                step_from: &step.from.id,
                upward: true,
                types: &step.target_types,
                layout: &self.layout,
            };
            for ty in &step.types {
                for (conversion, source, target) in
                    [(&ty.upcast, &old, &new), (&ty.downcast, &new, &old)]
                {
                    if let Some(conversion) = conversion {
                        self.conversion(
                            conversion,
                            ConversionInput {
                                key: &ty.key,
                                declaration: Some(&ty.key),
                                source,
                                target,
                                source_type: source.named(&ty.key),
                                target_type: target.named(&ty.key),
                                source_shape: Shape::Struct,
                                target_shape: Shape::Struct,
                            },
                            &mut file,
                        )?;
                    }
                }
            }
            for operation in &step.operations {
                self.operation(operation, (&old, &new), &mut file)?;
            }
        }
        self.endpoints(&mut file)?;
        self.wire(&mut file)?;
        for conversion in &file.conversions {
            for copy in &conversion.copies {
                copy.value.helpers(&mut file.helpers);
            }
            if let Some(value) = &conversion.value {
                value.helpers(&mut file.helpers);
            }
        }
        let mut symbols = BTreeSet::new();
        for name in file
            .decisions
            .iter()
            .chain(&file.requests)
            .map(|item| &item.name)
            .chain(file.conversions.iter().map(|item| &item.name))
            .chain(file.wire.iter().map(|item| &item.name))
            .chain(
                file.types
                    .iter()
                    .flat_map(|item| [&item.name, &item.function]),
            )
            .chain(file.dispatches.iter().map(|item| &item.outcome))
        {
            if !symbols.insert(name) {
                return Err(Layout::error(name, "generated symbol is ambiguous"));
            }
        }
        for names in [
            file.versions
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>(),
            file.hooks.iter().map(|item| &item.name).collect(),
        ] {
            let mut unique = BTreeSet::new();
            for name in names {
                if !unique.insert(name) {
                    return Err(Layout::error(
                        name,
                        "generated variant or hook name is ambiguous",
                    ));
                }
            }
        }
        Ok(file)
    }

    fn field_type(
        &self,
        member: &Member,
        names: &Names<'_>,
        declaration: Option<&str>,
    ) -> IdlcResult<String> {
        let recursive = match declaration {
            Some(key) => self
                .layout
                .recursive(&format!("{}::{key}", names.version), &member.name)?,
            None => false,
        };
        let value = member
            .value
            .as_ref()
            .ok_or_else(|| Layout::error(&member.name, "member has no value type"))?;
        names.value_type(value, recursive)
    }
}

struct ConversionInput<'a> {
    key: &'a str,
    declaration: Option<&'a str>,
    source: &'a Names<'a>,
    target: &'a Names<'a>,
    source_type: String,
    target_type: String,
    source_shape: Shape,
    target_shape: Shape,
}
