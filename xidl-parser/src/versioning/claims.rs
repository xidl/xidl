//! Wire-name claim validation across the version timeline.
//!
//! Renames are encoded as disjoint intervals, so one container legitimately
//! holds many declarations; overlapping intervals resolving to the same
//! effective wire name would emit one JSON key twice and are rejected.
//! `@skip` declarations never reach the wire and claim nothing.

use std::collections::HashMap;

use super::VersioningError;
use super::lifecycle::{Availability, Site, SiteKind, availability};
use super::surface::{declarator_ident, wire_name_of};
use super::timeline::VersionTimeline;
use crate::error::ParserResult;
use crate::hir::{Definition, Specification, TypeDcl, is_skipped};

/// One wire-name claim a declaration makes inside its container.
struct WireClaim {
    /// Declaration path, e.g. `Pet::age`; reported by collisions.
    decl: String,
    /// Effective wire name under the canonical rename rule.
    wire: String,
    from: usize,
    until: usize,
}

/// Collects the wire-name claims of every lifecycle-carrying container.
pub(crate) struct ClaimWalker<'a> {
    timeline: &'a VersionTimeline,
    containers: HashMap<String, Vec<WireClaim>>,
}

impl ClaimWalker<'_> {
    /// Validates that effective wire names stay unique per version inside
    /// every container that serializes them: structs, exceptions, unions and
    /// enums.
    ///
    /// Renames are encoded as disjoint intervals, so one container
    /// legitimately holds many declarations; overlapping intervals resolving
    /// to the same effective wire name would emit one JSON key twice and are
    /// rejected. `@skip` declarations never reach the wire and claim nothing.
    /// This runs on every compile: validation never depends on a projection
    /// being requested.
    pub(crate) fn check(spec: &Specification, timeline: &VersionTimeline) -> ParserResult<()> {
        let mut walker = ClaimWalker {
            timeline,
            containers: HashMap::new(),
        };
        walker.walk(&spec.0, "", Availability::full())?;
        walker.report()
    }

    fn report(self) -> ParserResult<()> {
        for (container, claims) in &self.containers {
            for (index, first) in claims.iter().enumerate() {
                for second in claims.iter().skip(index + 1) {
                    if first.wire != second.wire {
                        continue;
                    }
                    let overlap_from = first.from.max(second.from);
                    if overlap_from >= first.until.min(second.until) {
                        continue;
                    }
                    let version = self
                        .timeline
                        .iter()
                        .find(|(id, _)| id.index() == overlap_from)
                        .map(|(_, def)| def.value.clone())
                        .unwrap_or_default();
                    return Err(VersioningError::DuplicateWireName {
                        container: container.clone(),
                        wire: first.wire.clone(),
                        first: first.decl.clone(),
                        second: second.decl.clone(),
                        version,
                    }
                    .into());
                }
            }
        }
        Ok(())
    }

    fn walk(
        &mut self,
        definitions: &[Definition],
        path: &str,
        parent: Availability,
    ) -> ParserResult<()> {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    let here = super::paths::join(path, &module.ident);
                    let site = Site::new(SiteKind::Module, &here, path, &module.annotations);
                    let inner = availability(&site, parent, self.timeline)?;
                    self.walk(&module.definition, &here, inner)?;
                }
                Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                    self.walk_type(constr, path, parent)?;
                }
                Definition::ExceptDcl(except) => {
                    let here = super::paths::join(path, &except.ident);
                    let site = Site::new(SiteKind::Type, &here, path, &except.annotations);
                    let inner = availability(&site, parent, self.timeline)?;
                    let members = &except.member;
                    let annotations = &except.annotations;
                    self.claim_members(&here, inner, annotations, members)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn walk_type(
        &mut self,
        constr: &crate::hir::ConstrTypeDcl,
        path: &str,
        parent: Availability,
    ) -> ParserResult<()> {
        match constr {
            crate::hir::ConstrTypeDcl::StructDcl(def) => {
                let here = super::paths::join(path, &def.ident);
                let site = Site::new(SiteKind::Type, &here, path, &def.annotations);
                let inner = availability(&site, parent, self.timeline)?;
                self.claim_members(&here, inner, &def.annotations, &def.member)?;
            }
            crate::hir::ConstrTypeDcl::EnumDcl(def) => {
                let here = super::paths::join(path, &def.ident);
                let site = Site::new(SiteKind::Type, &here, path, &def.annotations);
                let inner = availability(&site, parent, self.timeline)?;
                for member in &def.member {
                    let member_site = Site::new(
                        SiteKind::Enumerator,
                        &super::paths::join(&here, &member.ident),
                        &here,
                        &member.annotations,
                    );
                    let member_avail = availability(&member_site, inner, self.timeline)?;
                    let wire = wire_name_of(&member.ident, &member.annotations, &def.annotations);
                    self.push(&here, member_site.path, wire, member_avail);
                }
            }
            crate::hir::ConstrTypeDcl::UnionDef(def) => {
                let here = super::paths::join(path, &def.ident);
                let site = Site::new(SiteKind::Type, &here, path, &def.annotations);
                let inner = availability(&site, parent, self.timeline)?;
                for case in &def.case {
                    let Some(name) = declarator_ident(&case.element.value) else {
                        continue;
                    };
                    let member_site = Site::new(
                        SiteKind::Member,
                        &super::paths::join(&here, &name),
                        &here,
                        &case.element.annotations,
                    );
                    let member_avail = availability(&member_site, inner, self.timeline)?;
                    let wire = wire_name_of(&name, &case.element.annotations, &def.annotations);
                    self.push(&here, member_site.path, wire, member_avail);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn claim_members(
        &mut self,
        container: &str,
        container_avail: Availability,
        container_annotations: &[crate::hir::Annotation],
        members: &[crate::hir::Member],
    ) -> ParserResult<()> {
        for member in members {
            if is_skipped(&member.annotations) {
                continue;
            }
            let site = Site::new(
                SiteKind::Member,
                &super::paths::join(container, &super::paths::declarators_name(&member.ident)),
                container,
                &member.annotations,
            );
            let member_avail = availability(&site, container_avail, self.timeline)?;
            for decl in &member.ident {
                let Some(name) = declarator_ident(decl) else {
                    continue;
                };
                let wire = wire_name_of(&name, &member.annotations, container_annotations);
                self.push(
                    container,
                    super::paths::join(container, &name),
                    wire,
                    member_avail,
                );
            }
        }
        Ok(())
    }

    fn push(&mut self, container: &str, decl: String, wire: String, avail: Availability) {
        self.containers
            .entry(container.to_string())
            .or_default()
            .push(WireClaim {
                decl,
                wire,
                from: avail.from,
                until: avail.until,
            });
    }
}
