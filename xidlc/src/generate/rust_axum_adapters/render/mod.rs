//! Assemble Rust adapter contexts without parser-internal lifecycle structures.

mod conversions;
mod endpoints;
mod operations;
mod stored;
mod unions;
mod update;
mod wire;

use super::context::*;
use super::layout::{Declaration, Layout};
use super::names::Names;
use crate::error::IdlcResult;
use crate::generate::adapter::{AdapterPlan, Member, StepUpdates, TypeConversion};
use crate::generate::rust::util::rust_ident;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;
use update::{UpdateMark, UpdateMarks};
use xidl_parser::rest_hir::RestHirDocument;

pub(super) struct AdapterRenderer<'a> {
    pub http: &'a RestHirDocument,
    pub models: &'a crate::generate::rust::RustRenderer,
    pub plan: &'a AdapterPlan,
    pub layout: Layout<'a>,
}

impl<'a> AdapterRenderer<'a> {
    pub(super) fn new(
        http: &'a RestHirDocument,
        plan: &'a AdapterPlan,
        models: &'a crate::generate::rust::RustRenderer,
    ) -> Self {
        Self {
            http,
            models,
            plan,
            layout: Layout::new(&http.spec),
        }
    }

    pub(super) fn context(&self) -> IdlcResult<FileContext> {
        let mut file = FileContext {
            has_updates: !self.plan.updates.operations.is_empty(),
            ..FileContext::default()
        };
        for version in &self.plan.versions {
            file.versions.push(Version {
                id: version.version.id.to_case(Case::Pascal),
                module: rust_ident(&version.version.id),
                raw_id: version.version.id.clone(),
                value: version.version.value.clone(),
            });
            let names = Names {
                models: self.models,
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
        let head_names = self.head_names();
        for (index, step) in self.plan.steps.iter().enumerate() {
            let old = Names {
                models: self.models,
                version: &step.from.id,
                step_from: &step.from.id,
                upward: false,
                types: &step.source_types,
                layout: &self.layout,
            };
            let new = Names {
                models: self.models,
                version: &step.to.id,
                step_from: &step.from.id,
                upward: true,
                types: &step.target_types,
                layout: &self.layout,
            };
            let marks = UpdateMarks::new(
                &self.plan.updates.steps[index],
                &self.plan.updates,
                &head_names,
            );
            for ty in &step.types {
                for (conversion, source, target, upward) in [
                    (&ty.upcast, &old, &new, true),
                    (&ty.downcast, &new, &old, false),
                ] {
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
                                update: upward.then(|| marks.of(&ty.key)),
                            },
                            &mut file,
                        )?;
                    }
                }
            }
            for operation in &step.operations {
                self.operation(index, operation, (&old, &new, &marks), &mut file)?;
            }
        }
        self.thread_stored(&mut file);
        let empty_step = StepUpdates::default();
        let head_names = self.head_names();
        let all_marks = UpdateMarks::new(&empty_step, &self.plan.updates, &head_names);
        for key in self.plan.updates.operations.keys() {
            let Some(bag) = all_marks.bag(self, key)? else {
                continue;
            };
            file.update_stored.push(bag);
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
            .chain(file.endpoints.iter().map(|item| &item.result))
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
            file.readers.iter().map(|item| &item.name).collect(),
        ] {
            let mut unique = BTreeSet::new();
            for name in names {
                if !unique.insert(name) {
                    return Err(Layout::error(
                        name,
                        "generated variant, hook, or stored value reader name is ambiguous",
                    ));
                }
            }
        }
        Ok(file)
    }

    /// Names of the Head surface, for update `current` and bag types.
    fn head_names(&self) -> Names<'_> {
        let head = self.plan.versions.last().expect("timeline is non-empty");
        Names {
            models: self.models,
            version: &head.version.id,
            step_from: &head.version.id,
            upward: false,
            types: &head.surface.types,
            layout: &self.layout,
        }
    }

    /// The member's type without its optionality, for stored bag fields.
    pub(super) fn bare_field_type(&self, member: &Member, names: &Names<'_>) -> IdlcResult<String> {
        let mut stripped = member.clone();
        if let Some(value) = &mut stripped.value {
            value.optional = false;
        }
        self.field_type(&stripped, names, None)
    }

    fn field_type(
        &self,
        member: &Member,
        names: &Names<'_>,
        declaration: Option<&str>,
    ) -> IdlcResult<String> {
        let recursive = match declaration {
            Some(key) => self.member_recursive(member, names, key)?,
            None => false,
        };
        let value = member
            .value
            .as_ref()
            .ok_or_else(|| Layout::error(&member.name, "member has no value type"))?;
        names.value_type(value, recursive)
    }

    fn member_recursive(&self, member: &Member, names: &Names<'_>, key: &str) -> IdlcResult<bool> {
        let owner = member.path.last().map(String::as_str).unwrap_or(key);
        self.layout
            .recursive(&format!("{}::{owner}", names.version), &member.name)
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
    update: Option<UpdateMark>,
}
