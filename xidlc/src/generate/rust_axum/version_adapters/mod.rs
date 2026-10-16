//! Rust rendering of the version-adapter plan for rust-axum.
//!
//! Rust assembles names, type expressions and decision shapes; the
//! minijinja template renders the statements. The adapter module is
//! appended to the contract's generated file, so it references the
//! version modules through `{version}::` paths imported from `super`.

mod assembly;
mod context;
mod naming;
mod operations;

use crate::error::IdlcResult;
use crate::generate::adapter::AdapterPlan;

use context::{DecisionContext, HookMember, ModuleContext};

/// Decision and hook names accumulated across the whole plan.
#[derive(Default)]
pub(super) struct RenderState {
    pub(super) hooks: Vec<HookMember>,
    pub(super) decisions: Vec<DecisionContext>,
}

/// Renders the version-adapter module body for the plan.
pub(crate) fn render_module(
    plan: &AdapterPlan,
    renderer: &crate::generate::rust_axum::RustAxumRenderer,
) -> IdlcResult<String> {
    let mut state = RenderState::default();
    let mut versions = Vec::new();
    let mut requests = Vec::new();
    let mut conversions = Vec::new();
    let mut errors = Vec::new();
    let mut results = Vec::new();
    for step in &plan.steps {
        versions.push(step.from.ident.clone());
        versions.push(step.to.ident.clone());
        let names = assembly::StepNames::new(step);
        names.push_type_conversions(&mut state, &mut conversions)?;
        names.push_operation_conversions(
            step,
            &mut state,
            &mut requests,
            &mut conversions,
            &mut errors,
            &mut results,
        )?;
    }
    versions.sort();
    versions.dedup();
    let context = ModuleContext {
        versions,
        hooks: state.hooks,
        decisions: state.decisions,
        requests,
        conversions,
        errors,
        results,
    };
    renderer.render_template("version_adapters.rs.j2", &context)
}
