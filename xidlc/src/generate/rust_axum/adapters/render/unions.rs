//! Project only union layouts whose public constructors preserve the tag.

use super::*;
use crate::generate::adapter::ConversionPlan;
use crate::generate::rust::util::declarator_name;
use crate::generate::rust::util::http_union_case_ident;
use xidl_parser::hir;

impl AdapterRenderer<'_> {
    pub(super) fn union(
        &self,
        plan: &ConversionPlan,
        input: &ConversionInput<'_>,
        output: &mut Conversion,
        file: &mut FileContext,
    ) -> IdlcResult<()> {
        let source = self.union_definition(input.source, input.key)?;
        let target = self.union_definition(input.target, input.key)?;
        let is_http = crate::generate::utils::has_annotation(&source.annotations, "http");
        if is_http != crate::generate::utils::has_annotation(&target.annotations, "http") {
            return Err(Layout::error(
                input.key,
                "union model representations differ",
            ));
        }
        if is_http {
            output.kind = "http_union";
        }
        for copy in &plan.copies {
            let old = self.union_case(source, &copy.source.name)?;
            let new = self.union_case(target, &copy.target.name)?;
            output.copies.push(Copy {
                source: self.union_case_name(old, is_http)?,
                target: self.union_case_name(new, is_http)?,
                value: input
                    .target
                    .field_value(&copy.value, (old.element.recursive, new.element.recursive)),
            });
        }
        for member in &plan.consumed {
            let case = self.union_case(source, &member.name)?;
            let value = member
                .value
                .as_ref()
                .ok_or_else(|| Layout::error(input.key, "union case has no public value type"))?;
            output.changed.push(ChangedVariant {
                name: self.union_case_name(case, is_http)?,
                ty: Some(input.source.value_type(value, case.element.recursive)?),
            });
        }
        if !output.changed.is_empty() {
            let name = input.target.hook(input.key);
            file.hooks.push(Hook {
                name: name.clone(),
                source: format!("{}<'_>", output.changed_type),
                target: input.target_type.clone(),
            });
            output.hook = Some(name);
        }
        Ok(())
    }

    pub(super) fn union_definition<'a>(
        &'a self,
        names: &Names<'_>,
        key: &str,
    ) -> IdlcResult<&'a hir::UnionDef> {
        let raw = format!("{}::{key}", names.version);
        match self.layout.declarations.get(&raw) {
            Some(Declaration::Union(value)) => Ok(value),
            _ => Err(Layout::error(&raw, "missing public union declaration")),
        }
    }

    fn union_case<'a>(
        &self,
        declaration: &'a hir::UnionDef,
        name: &str,
    ) -> IdlcResult<&'a hir::Case> {
        declaration
            .case
            .iter()
            .find(|case| declarator_name(&case.element.value) == name)
            .ok_or_else(|| Layout::error(&declaration.ident, "union case has no public layout"))
    }

    pub(super) fn union_case_name(&self, case: &hir::Case, is_http: bool) -> IdlcResult<String> {
        if hir::is_skipped(&case.element.annotations) {
            return Err(Layout::error(
                &declarator_name(&case.element.value),
                "skipped union cases have no adapter layout",
            ));
        }
        let [hir::CaseLabel::Value(label)] = case.label.as_slice() else {
            return Err(Layout::error(
                &declarator_name(&case.element.value),
                "multiple or default union tags cannot be preserved by the ordinary public constructors",
            ));
        };
        if is_http {
            let hir::ConstExpr::ScopedName(name) = label else {
                return Err(Layout::error(
                    "HTTP union",
                    "case requires a ContentType label",
                ));
            };
            let ident = name
                .name
                .last()
                .ok_or_else(|| Layout::error("HTTP union", "empty ContentType label"))?;
            Ok(http_union_case_ident(ident))
        } else {
            Ok(rust_ident(&declarator_name(&case.element.value)))
        }
    }
}
