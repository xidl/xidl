//! The version spine: one derived artifact relating every declared version.
//!
//! For each version the spine records the canonical wire-surface digest; for
//! each adjacent version pair it records how every wire fact carried over,
//! classified with the double-signal continuity rule: a `Renamed` cell
//! requires both an explicit `@rename` on the new declaration and a wire-name
//! trajectory that confirms it. Wire coincidences never become rename facts —
//! they land on the review side (`NameShifted`, or `Added` with resume
//! evidence), so generated adapters can never silently forge an old
//! contract's semantics.

use std::collections::HashMap;

use serde::Serialize;

use super::surface::VersionSurface;
use super::{VersionDef, VersionId, VersioningError, VersioningRecord};
use crate::error::ParserResult;
use crate::hir::Specification;

/// How one wire fact carried between two adjacent versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CellKind {
    /// Same wire name, same type, same optionality, same logical name.
    Mechanical,
    /// Wire-equal continuation whose logical name changed under an explicit,
    /// trajectory-confirmed `@rename`.
    Renamed,
    /// Wire-equal continuation whose logical name changed without rename
    /// evidence; a coincidence that requires review before adapters rely on
    /// it.
    NameShifted,
    /// Same wire name with a different type.
    TypeChanged,
    /// Same wire name with changed optionality.
    Optionalized,
    /// No counterpart in the adjacent previous version.
    Added,
    /// The adjacent previous version carried it and this one does not.
    Removed,
}

/// Why the spine classified a cell the way it did; kept on the cell so every
/// classification stays auditable instead of becoming a black box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    /// Present in both versions with no observable delta.
    Same,
    /// The new declaration carries an explicit `@rename` whose value the wire
    /// name confirms.
    ExplicitRename,
    /// Wire names match but no rename was declared; a coincidence.
    WireNameMatch,
    /// The wire name existed earlier, went away, and resumed `gap` versions
    /// later; `confirmed` records whether an explicit `@rename` declared the
    /// continuation.
    ResumesAfterGap { gap: usize, confirmed: bool },
    /// First appearance at the start of the timeline.
    IntervalStart,
    /// Interval end inside the timeline.
    IntervalEnd,
}

/// One correspondence cell of an adjacent version pair.
#[derive(Debug, Clone, Serialize)]
pub struct CorrespondenceCell {
    /// Version the fact came from (member ident).
    pub from: String,
    /// Version the fact was compared against (member ident).
    pub to: String,
    /// Declaration path in the earlier version, if it existed there.
    pub from_subject: Option<String>,
    /// Declaration path in the later version, if it exists there.
    pub to_subject: Option<String>,
    /// Container path, stable across versions for the join.
    pub container: String,
    /// Effective wire name under the canonical rename rule.
    pub wire_name: String,
    /// How the fact carried over.
    pub kind: CellKind,
    /// Audit trail of the classification.
    pub evidence: Evidence,
}

/// Digest summary of one version's canonical wire surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfaceSummary {
    /// The version the surface belongs to.
    pub version: VersionDef,
    /// Canonical digest of the projected wire surface.
    pub digest: String,
    /// Number of wire facts on the surface.
    pub entries: usize,
}

/// The derived version spine of a versioned specification.
#[derive(Debug, Clone, Serialize)]
pub struct Spine {
    /// Ordered version history, as declared.
    pub timeline: Vec<VersionDef>,
    /// Per-version canonical wire-surface digest, in timeline order.
    pub surfaces: Vec<SurfaceSummary>,
    /// How every wire fact carried between adjacent versions.
    pub correspondence: Vec<CorrespondenceCell>,
}

impl Spine {
    /// Derives the spine: projects every declared version, digests each
    /// canonical surface and joins adjacent versions on
    /// `(container, wire name)` under the double-signal continuity rule.
    pub fn build(record: &VersioningRecord, source: &Specification) -> ParserResult<Spine> {
        let mut surfaces = Vec::new();
        let mut summaries = Vec::new();
        for (id, def) in record.timeline().iter() {
            let projected = record.project(source, id)?;
            let surface = VersionSurface::of(&projected, &def.ident);
            summaries.push(SurfaceSummary {
                version: def.clone(),
                digest: surface.digest(),
                entries: surface.entries().len(),
            });
            surfaces.push(surface);
        }

        let mut correspondence = Vec::new();
        for window in 1..surfaces.len() {
            let context = PairContext {
                record,
                earlier: &surfaces[..window],
                previous: &surfaces[window - 1],
                current: &surfaces[window],
                from: VersionId::from_index(window - 1),
                to: VersionId::from_index(window),
                cells: &mut correspondence,
            };
            context.join()?;
        }

        Ok(Spine {
            timeline: record
                .timeline()
                .iter()
                .map(|(_, def)| def.clone())
                .collect(),
            surfaces: summaries,
            correspondence,
        })
    }
}

/// One adjacent version pair and everything the join needs.
struct PairContext<'a> {
    record: &'a VersioningRecord,
    /// Every version surface before the pair's `from`, for gap detection.
    earlier: &'a [VersionSurface],
    previous: &'a VersionSurface,
    current: &'a VersionSurface,
    from: VersionId,
    to: VersionId,
    cells: &'a mut Vec<CorrespondenceCell>,
}

impl PairContext<'_> {
    fn join(self) -> ParserResult<()> {
        let Self {
            record,
            earlier,
            previous,
            current,
            from,
            to,
            cells,
        } = self;
        let timeline_name = |id: VersionId| {
            record
                .timeline()
                .get(id)
                .map(|def| def.ident.clone())
                .ok_or_else(|| VersioningError::UnknownVersion { index: id.index() })
        };
        let from_name = timeline_name(from)?;
        let to_name = timeline_name(to)?;
        let previous_index = previous.index();
        let current_index = current.index();
        let mut last_seen = HashMap::new();
        for (position, surface) in earlier.iter().enumerate() {
            for entry in surface.entries() {
                last_seen.insert(
                    (entry.container.as_str(), entry.wire_name.as_str()),
                    position,
                );
            }
        }

        for entry in self.current.entries() {
            let key = (entry.container.as_str(), entry.wire_name.as_str());
            let Some(prior_position) = previous_index.get(&key) else {
                let confirmed = entry.explicit_rename.as_deref() == Some(entry.wire_name.as_str());
                let evidence = match last_seen.remove(&key) {
                    Some(position) => Evidence::ResumesAfterGap {
                        gap: self.from.index() - position,
                        confirmed,
                    },
                    None => Evidence::IntervalStart,
                };
                cells.push(CorrespondenceCell {
                    from: from_name.clone(),
                    to: to_name.clone(),
                    from_subject: None,
                    to_subject: Some(entry.subject.clone()),
                    container: entry.container.clone(),
                    wire_name: entry.wire_name.clone(),
                    kind: CellKind::Added,
                    evidence,
                });
                continue;
            };
            let prior = &self.previous.entries()[*prior_position];
            let (kind, evidence) = if prior.type_repr != entry.type_repr {
                (CellKind::TypeChanged, Evidence::Same)
            } else if prior.optional != entry.optional {
                (CellKind::Optionalized, Evidence::Same)
            } else if prior.subject != entry.subject {
                match entry.explicit_rename.as_deref() {
                    Some(wire) if wire == entry.wire_name => {
                        (CellKind::Renamed, Evidence::ExplicitRename)
                    }
                    _ => (CellKind::NameShifted, Evidence::WireNameMatch),
                }
            } else {
                (CellKind::Mechanical, Evidence::Same)
            };
            cells.push(CorrespondenceCell {
                from: from_name.clone(),
                to: to_name.clone(),
                from_subject: Some(prior.subject.clone()),
                to_subject: Some(entry.subject.clone()),
                container: entry.container.clone(),
                wire_name: entry.wire_name.clone(),
                kind,
                evidence,
            });
        }

        for entry in self.previous.entries() {
            let key = (entry.container.as_str(), entry.wire_name.as_str());
            if current_index.contains_key(&key) {
                continue;
            }
            cells.push(CorrespondenceCell {
                from: from_name.clone(),
                to: to_name.clone(),
                from_subject: Some(entry.subject.clone()),
                to_subject: None,
                container: entry.container.clone(),
                wire_name: entry.wire_name.clone(),
                kind: CellKind::Removed,
                evidence: Evidence::IntervalEnd,
            });
        }
        Ok(())
    }
}
