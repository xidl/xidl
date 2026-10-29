//! Splicing `#include "<path>@<version>"` as that version's projection.

use super::spec::collect_defs;
use super::{Definition, Pragma, Specification, split_version_selector};
use crate::error::{ParseError, ParserResult};
use crate::parser::{IncludeResolver, parser_text_with_resolver};
use crate::versioning::VersioningRecord;

/// Expands versioned includes while a specification is lowered.
///
/// A plain `#include` is copied at the parse stage. A versioned include keeps
/// its directive in the tree, so this expander can read the unit, project it at
/// the requested version and splice the projection in place. The projection
/// carries the version module of the included unit, which is what references
/// such as `v1::Instant` name.
pub(crate) struct VersionedIncludes<'a> {
    resolver: &'a mut dyn IncludeResolver,
    current: Option<String>,
    stack: Vec<String>,
}

impl<'a> VersionedIncludes<'a> {
    /// Creates an expander rooted at `current`, the file being lowered.
    pub(crate) fn new(resolver: &'a mut dyn IncludeResolver, current: Option<String>) -> Self {
        Self {
            resolver,
            current,
            stack: Vec::new(),
        }
    }

    /// Splices the projection `raw_path` requests into `out`.
    ///
    /// A path without a version selector was already copied at the parse stage
    /// and needs nothing here.
    ///
    /// # Errors
    ///
    /// Returns an error when the included unit cannot be read, declares no
    /// version timeline, or declares no version matching the selector.
    pub(crate) fn expand(&mut self, raw_path: &str, out: &mut Vec<Definition>) -> ParserResult<()> {
        let (target, version) = split_version_selector(raw_path);
        let Some(version) = version else {
            return Ok(());
        };
        let (file, content) = self.resolver.resolve(self.current.as_deref(), target)?;
        if self.stack.contains(&file) {
            return Err(ParseError::Message(format!(
                "cyclic include detected: {} -> {file}",
                self.stack.join(" -> ")
            )));
        }
        self.stack.push(file.clone());
        let projected = self.project(&file, &content, version);
        self.stack.pop();
        out.extend(projected?);
        Ok(())
    }

    /// Projects one included unit at `version`.
    fn project(
        &mut self,
        file: &str,
        content: &str,
        version: &str,
    ) -> ParserResult<Vec<Definition>> {
        let typed = parser_text_with_resolver(content, Some(file), &mut *self.resolver)?;
        crate::lint::AnnotationKind::Webhooks.validate_source(&typed.0)?;
        let current = self.current.replace(file.to_string());
        let mut definitions = Vec::new();
        let collected = collect_defs(typed.0, &mut definitions, self);
        self.current = current;
        collected?;

        let spec = Specification(definitions);
        let Some(record) = VersioningRecord::analyze(&spec)? else {
            return Err(ParseError::Message(format!(
                "xidl include: {file} is included at version `{version}` and declares no \
                 version timeline"
            )));
        };
        let timeline = record.timeline();
        let id = timeline
            .iter()
            .find(|(_, definition)| definition.ident == version || definition.value == version)
            .map(|(id, _)| id);
        let Some(id) = id else {
            return Err(ParseError::Message(format!(
                "xidl include: {file} declares no version `{version}` (declared versions: {})",
                timeline.declared()
            )));
        };
        let projected = record.project(&spec, id)?;
        // The projection carries the document version of the included unit. It
        // belongs to that unit, so it stays out of the including one.
        Ok(projected
            .0
            .into_iter()
            .filter(|definition| {
                !matches!(
                    definition,
                    Definition::Pragma(Pragma::XidlcOpenApiVersion(_))
                )
            })
            .collect())
    }
}
