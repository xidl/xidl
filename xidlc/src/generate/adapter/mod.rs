//! Language-neutral conversion plans built from public version-map facts.

mod classify;
mod collect;
mod facts;
mod model;
mod planner;
mod references;

pub(crate) use model::*;
pub(crate) use planner::AdapterPlanner;
