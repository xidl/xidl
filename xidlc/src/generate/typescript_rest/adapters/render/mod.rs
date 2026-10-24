//! Assemble TypeScript contexts from language-neutral adjacent plans.

mod conversion;
mod operation;
mod stored;

use super::context::*;
use super::names::Names;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{
    AdapterPlan, ConversionPlan, Member, OperationPlan, StepPlan, TypeConversion,
};
use crate::generate::typescript::definition::contexts::{FieldTypeContext, TsType};
use crate::generate::typescript::definition::names::{
    method_struct_prefix, scoped_name, ts_prop_name,
};
use crate::generate::typescript_rest::model::TsRaisesContext;
use conversion::ConversionInput;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;
use xidl_parser::rest_hir::RestHirDocument;

pub(super) struct AdapterRenderer<'a> {
    pub(super) http: &'a RestHirDocument,
    pub(super) plan: &'a AdapterPlan,
}

impl<'a> AdapterRenderer<'a> {
    pub(super) fn new(http: &'a RestHirDocument, plan: &'a AdapterPlan) -> Self {
        Self { http, plan }
    }

    pub(super) fn context(&self, stem: &str) -> IdlcResult<FileContext> {
        let mut file = FileContext {
            file_stem: stem.into(),
            has_errors: false,
            decisions: Vec::new(),
            hooks: Vec::new(),
            readers: Vec::new(),
            needs: Vec::new(),
            conversions: Vec::new(),
            operations: Vec::new(),
            wire_definitions: Vec::new(),
            endpoints: Vec::new(),
            dispatchers: Vec::new(),
            type_dispatchers: Vec::new(),
        };
        for step in &self.plan.steps {
            let old_keys = step.source_types.keys().cloned().collect::<BTreeSet<_>>();
            let new_keys = step.target_types.keys().cloned().collect::<BTreeSet<_>>();
            let old = Names {
                keys: &old_keys,
                version: &step.from.id,
                from: &step.from.id,
                upward: false,
            };
            let new = Names {
                keys: &new_keys,
                version: &step.to.id,
                from: &step.from.id,
                upward: true,
            };
            for ty in &step.types {
                for (conversion, source, target) in
                    [(&ty.upcast, &old, &new), (&ty.downcast, &new, &old)]
                {
                    let Some(conversion) = conversion else {
                        continue;
                    };
                    let request = ConversionInput {
                        key: &ty.key,
                        target,
                        source,
                        source_type: source.named(&ty.key),
                        target_type: target.named(&ty.key),
                    };
                    self.conversion(conversion, request, &mut file)?;
                }
            }
            for operation in &step.operations {
                self.operation(step, operation, (&old, &new), &mut file)?;
            }
        }
        self.thread_stored(&mut file);
        self.endpoints(&mut file)?;
        self.type_endpoints(&mut file)?;
        file.has_errors = file
            .endpoints
            .iter()
            .any(|endpoint| !endpoint.errors.is_empty() || !endpoint.head_errors.is_empty());
        file.validate_symbols()?;
        Ok(file)
    }

    pub(super) fn object_type(&self, members: &[Member], names: &Names<'_>) -> IdlcResult<TsType> {
        Ok(TsType::InlineStruct(
            members
                .iter()
                .map(|member| {
                    Ok(FieldTypeContext {
                        prop: ts_prop_name(&member.name),
                        ty: names.member_type(member)?,
                        optional: member.value.as_ref().is_some_and(Names::optional),
                        doc: Vec::new(),
                    })
                })
                .collect::<IdlcResult<Vec<_>>>()?,
        ))
    }

    pub(super) fn error_class(&self, key: &str, version: &str) -> IdlcResult<String> {
        let full = format!("{version}::{key}");
        let exceptions = &self.http.document.exceptions;
        let exception = exceptions
            .iter()
            .find(|exception| {
                exception
                    .module_path
                    .iter()
                    .map(String::as_str)
                    .chain(std::iter::once(exception.ident.as_str()))
                    .collect::<Vec<_>>()
                    .join("::")
                    == full
            })
            .ok_or_else(|| {
                IdlcError::rpc(format!("typescript adapters: missing exception {full}"))
            })?;
        Ok(TsRaisesContext::error_ident(exception, exceptions))
    }
}
