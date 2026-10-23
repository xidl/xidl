//! Build adjacent adapter plans from canonical facts and typed HTTP shapes.

mod step;

use super::classify::{Direction, MemberClassifier, MemberSetKind};
use super::collect::SurfaceCollector;
use super::facts::{MapFacts, ParameterSide};
use super::model::*;
use super::references::ReferencePlanner;
use crate::error::{IdlcError, IdlcResult};
use std::collections::{BTreeMap, BTreeSet};
use step::StepPlanner;
use xidl_parser::hir::ParserProperties;
use xidl_parser::rest_hir::RestHirDocument;
use xidl_parser::versioning::map::{Change, ChangeKind, MapStep, MapVersion, VersionMap};

pub(crate) struct AdapterPlanner<'a> {
    http: &'a RestHirDocument,
    map: VersionMap,
}

impl<'a> AdapterPlanner<'a> {
    pub(crate) fn from_props(
        http: &'a RestHirDocument,
        props: &ParserProperties,
    ) -> IdlcResult<Self> {
        Ok(Self {
            http,
            map: MapFacts::read(props)?,
        })
    }

    pub(crate) fn plan(&self) -> IdlcResult<AdapterPlan> {
        let surfaces = SurfaceCollector::collect(self.http, &self.map)?;
        let mut steps = Vec::new();
        for (index, facts) in self.map.steps.iter().enumerate() {
            let source = surfaces
                .get(&facts.from)
                .ok_or_else(|| self.error("missing source version surface"))?;
            let target = surfaces
                .get(&facts.to)
                .ok_or_else(|| self.error("missing target version surface"))?;
            let planner = StepPlanner {
                source,
                target,
                facts,
                contract: &self.map.contract,
                versions: (&self.map.versions[index], &self.map.versions[index + 1]),
            };
            let (types, operations) = planner.plan()?;
            steps.push(StepPlan {
                from: self.map.versions[index].clone(),
                to: self.map.versions[index + 1].clone(),
                source_types: source.types.clone(),
                target_types: target.types.clone(),
                types,
                operations,
            });
        }
        let versions = self
            .map
            .versions
            .iter()
            .map(|version| {
                let surface = surfaces
                    .get(&version.id)
                    .cloned()
                    .ok_or_else(|| self.error("missing complete version surface"))?;
                Ok(VersionPlan {
                    version: version.clone(),
                    surface,
                })
            })
            .collect::<IdlcResult<Vec<VersionPlan>>>()?;
        let needs = Self::needs(&steps, &self.map.versions);
        let updates = crate::generate::adapter::updates::plan(&steps, &versions)?;
        Ok(AdapterPlan {
            versions,
            steps,
            needs,
            updates,
        })
    }

    /// A step's downcast assignments are exactly the members it removes from a
    /// type or an operation response, so needs summarize validated plans.
    fn needs(
        steps: &[StepPlan],
        versions: &[MapVersion],
    ) -> BTreeMap<String, Vec<(String, Vec<String>)>> {
        let per_step = steps
            .iter()
            .map(|step| {
                let mut wires: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
                let types = step.types.iter().filter_map(|plan| match &plan.downcast {
                    Some(TypeConversion::Struct(conversion)) => {
                        Some((plan.key.as_str(), &conversion.assignments))
                    }
                    _ => None,
                });
                let operations = step
                    .operations
                    .iter()
                    .map(|plan| (plan.key.as_str(), &plan.downcast.assignments));
                for (key, members) in types.chain(operations) {
                    if members.is_empty() {
                        continue;
                    }
                    wires
                        .entry(key)
                        .or_default()
                        .extend(members.iter().map(|member| member.wire.as_str()));
                }
                wires
            })
            .collect::<Vec<_>>();
        let keys = per_step
            .iter()
            .flat_map(|wires| wires.keys().copied())
            .collect::<BTreeSet<_>>();
        keys.into_iter()
            .map(|key| {
                let entries = (0..versions.len())
                    .rev()
                    .map(|index| {
                        let names = per_step[index..]
                            .iter()
                            .filter_map(|wires| wires.get(key))
                            .flatten()
                            .map(|wire| wire.to_string())
                            .collect::<BTreeSet<_>>();
                        (versions[index].value.clone(), names.into_iter().collect())
                    })
                    .collect();
                (key.to_string(), entries)
            })
            .collect()
    }

    fn error(&self, reason: &str) -> IdlcError {
        IdlcError::rpc(format!(
            "version adapters for {}: {reason}",
            self.map.contract
        ))
    }
}
