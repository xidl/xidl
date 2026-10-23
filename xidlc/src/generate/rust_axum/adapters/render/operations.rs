//! Preserve ordinary request/result layouts and the complete typed error table.

use super::*;
use crate::generate::adapter::{Operation as SurfaceOperation, OperationPlan};
use crate::generate::rust_axum::interface::{RaisesContext, method_struct_prefix};

impl AdapterRenderer<'_> {
    pub(super) fn request(
        &self,
        operation: &SurfaceOperation,
        names: &Names<'_>,
    ) -> IdlcResult<Decision> {
        let fields = operation
            .inputs
            .iter()
            .map(|member| {
                Ok(Field {
                    name: rust_ident(&member.name),
                    ty: self.field_type(member, names, None)?,
                })
            })
            .collect::<IdlcResult<_>>()?;
        Ok(Decision {
            name: self.request_type(operation, names.version),
            fields,
        })
    }

    pub(super) fn request_type(&self, operation: &SurfaceOperation, version: &str) -> String {
        format!(
            "{}Request{}",
            Self::prefix(operation),
            version.to_case(Case::Pascal)
        )
    }

    pub(super) fn prefix(operation: &SurfaceOperation) -> String {
        let prefix = method_struct_prefix(
            operation
                .interface
                .rsplit("::")
                .next()
                .unwrap_or(&operation.interface),
            &operation.name,
        );
        format!(
            "{}{}",
            operation.scope.join("_").to_case(Case::Pascal),
            prefix
        )
    }

    pub(super) fn response_type(
        &self,
        operation: &SurfaceOperation,
        names: &Names<'_>,
    ) -> IdlcResult<(String, Shape)> {
        match operation.outputs.as_slice() {
            [] => Ok(("()".into(), Shape::Unit)),
            [member] if member.name == "return" => {
                Ok((self.field_type(member, names, None)?, Shape::Direct))
            }
            _ => {
                let prefix = method_struct_prefix(
                    operation
                        .interface
                        .rsplit("::")
                        .next()
                        .unwrap_or(&operation.interface),
                    &operation.name,
                );
                Ok((
                    names.named(&Layout::key(&operation.scope, &format!("{prefix}Response"))),
                    Shape::Struct,
                ))
            }
        }
    }

    pub(super) fn error_type(&self, operation: &SurfaceOperation, names: &Names<'_>) -> String {
        if operation.raises.is_empty() {
            return "::xidl_rust_axum::Error".into();
        }
        let prefix = method_struct_prefix(
            operation
                .interface
                .rsplit("::")
                .next()
                .unwrap_or(&operation.interface),
            &operation.name,
        );
        names.named(&Layout::key(&operation.scope, &format!("{prefix}Error")))
    }

    pub(super) fn operation(
        &self,
        index: usize,
        plan: &OperationPlan,
        names: (&Names<'_>, &Names<'_>, &UpdateMarks<'_>),
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        let (old, new, marks) = names;
        let mark = marks.envelope(&plan.key);
        let prefix = Self::prefix(&plan.source);
        let source_request = self.request_type(&plan.source, old.version);
        let target_request = self.request_type(&plan.target, new.version);
        let request_key = format!("{prefix}Request");
        self.conversion(
            &TypeConversion::Struct(plan.upcast.clone()),
            ConversionInput {
                key: &request_key,
                declaration: None,
                source: old,
                target: new,
                source_type: source_request.clone(),
                target_type: target_request.clone(),
                source_shape: Shape::Struct,
                target_shape: Shape::Struct,
                update: Some(mark.clone()),
            },
            file,
        )?;
        let (source_response, source_shape) = self.response_type(&plan.target, new)?;
        let (target_response, target_shape) = self.response_type(&plan.source, old)?;
        let response_key = format!("{prefix}Response");
        self.conversion(
            &TypeConversion::Struct(plan.downcast.clone()),
            ConversionInput {
                key: &response_key,
                declaration: None,
                source: new,
                target: old,
                source_type: source_response.clone(),
                target_type: target_response.clone(),
                source_shape,
                target_shape,
                update: None,
            },
            file,
        )?;
        let source_error = self.error_type(&plan.target, new);
        let target_error = self.error_type(&plan.source, old);
        let source_variants = RaisesContext::variant_names(&plan.target.http, self.http)
            .map_err(|error| Layout::error(&plan.key, &error.to_string()))?;
        let target_variants = RaisesContext::variant_names(&plan.source.http, self.http)
            .map_err(|error| Layout::error(&plan.key, &error.to_string()))?;
        let mut arms = Vec::new();
        for (key, variant) in plan.target.raises.iter().zip(source_variants) {
            if let Some(index) = plan.source.raises.iter().position(|old| old == key) {
                arms.push(ErrorArm {
                    source: variant,
                    target: Some(target_variants[index].clone()),
                    call: Some(Call {
                        name: old.function(key),
                        stored: false,
                        current: false,
                        pair: false,
                    }),
                    hook: None,
                });
            } else {
                let hook = old.hook(&format!("{prefix}_{key}Error"));
                file.hooks.push(Hook {
                    name: hook.clone(),
                    source: new.named(key),
                    target: target_error.clone(),
                    stored: false,
                    current: None,
                });
                arms.push(ErrorArm {
                    source: variant,
                    target: None,
                    call: None,
                    hook: Some(hook),
                });
            }
        }
        let _ = index;
        file.operations.push(Operation {
            current: mark.current,
            prefix: format!(
                "{}_from_{}",
                prefix.to_case(Case::Snake),
                old.version.to_case(Case::Snake)
            ),
            source_error,
            target_error,
            arms,
            source_typed_error: !plan.target.raises.is_empty(),
            target_typed_error: !plan.source.raises.is_empty(),
            stored: false,
        });
        Ok(())
    }
}
