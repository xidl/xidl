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
            .collect::<IdlcResult<_>>()?;
        Ok(AdapterPlan { versions, steps })
    }

    fn error(&self, reason: &str) -> IdlcError {
        IdlcError::rpc(format!(
            "version adapters for {}: {reason}",
            self.map.contract
        ))
    }
}
