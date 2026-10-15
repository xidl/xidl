//! Read-only existence intervals of every versioned declaration.
//!
//! The adapter stage is the one stage allowed to see several versions at
//! once; it derives its pairing from these intervals instead of re-reading
//! lifecycle annotations, which the projection strips from version modules.

use super::super::{IntervalKind, MemberInterval};
use super::super::timeline::VersionTimeline;
use super::availability::{Availability, Site, SiteKind, VersionVisitor, availability};
use crate::error::ParserResult;
use crate::hir::Definition;

/// Collects the interval of every declaration it visits.
struct IntervalCollector<'a> {
    timeline: &'a VersionTimeline,
    version_enum: String,
    intervals: Vec<MemberInterval>,
}

impl VersionVisitor for IntervalCollector<'_> {
    fn enter(&mut self, site: &Site<'_>, parent: Availability) -> ParserResult<Availability> {
        let interval = availability(site, parent, self.timeline)?;
        let in_version_enum = site.path == self.version_enum
            || site.path.starts_with(&format!("{}::", self.version_enum));
        if !in_version_enum {
            if let Some(kind) = interval_kind(site.kind) {
                self.intervals.push(MemberInterval {
                    path: site.path.clone(),
                    kind,
                    from: interval.from,
                    until: interval.until,
                });
            }
        }
        Ok(interval)
    }
}

/// Modules and interfaces only scope their children; declarations that
/// cannot carry lifecycle annotations always exist and carry no interval.
fn interval_kind(kind: SiteKind) -> Option<IntervalKind> {
    match kind {
        SiteKind::Type => Some(IntervalKind::Type),
        SiteKind::Member => Some(IntervalKind::Member),
        SiteKind::Enumerator => Some(IntervalKind::Enumerator),
        SiteKind::BitmaskValue => Some(IntervalKind::BitmaskValue),
        SiteKind::Operation => Some(IntervalKind::Operation),
        SiteKind::Parameter => Some(IntervalKind::Parameter),
        SiteKind::Attribute => Some(IntervalKind::Attribute),
        SiteKind::Module | SiteKind::Interface | SiteKind::Unsupported => None,
    }
}

/// Records the interval of every declaration under the given timeline,
/// excluding the `@version` enum itself: its members are the timeline, not
/// versioned data.
///
/// Fails with the same diagnostics as projection when the lifecycle
/// annotations are malformed, so callers can treat a successful call as a
/// validated specification.
pub(crate) fn collect_intervals(
    definitions: &[Definition],
    timeline: &VersionTimeline,
) -> ParserResult<Vec<MemberInterval>> {
    let mut visitor = IntervalCollector {
        timeline,
        version_enum: timeline.enum_ident().to_string(),
        intervals: Vec::new(),
    };
    visitor.walk(definitions, "", Availability::full())?;
    Ok(visitor.intervals)
}
