//! Splicing `#include "<path>@<version>"` as that version's projection.

use crate::diagnostic::DiagnosticBag;
use crate::error::{ParseError, ParserResult};
use crate::hir::{Definition, Pragma, Specification, VersionedIncludeExpander};
use crate::parser::{IncludeResolver, parser_text_with_resolver, split_version_selector};
use crate::pipeline::passes::SourceAstPasses;
use crate::versioning::VersioningRecord;

/// Expands versioned includes while a specification is lowered.
///
/// A plain `#include` is copied at the parse stage. A versioned include keeps
/// its directive in the tree, so this expander can read the unit, project it at
/// the requested version and splice the projection in place.
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

    /// Projects one included unit at `version`.
    fn project(
        &mut self,
        file: &str,
        content: &str,
        version: &str,
    ) -> ParserResult<Vec<Definition>> {
        let typed = parser_text_with_resolver(content, Some(file), &mut *self.resolver)?;
        let mut bag = DiagnosticBag::new();
        SourceAstPasses::run(&typed, &mut bag);
        bag.check_errors()?;

        let current = self.current.replace(file.to_string());
        let lowered = Specification::lower_raw(typed, self);
        self.current = current;
        let spec = lowered?;

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

impl VersionedIncludeExpander for VersionedIncludes<'_> {
    fn expand(&mut self, raw_path: &str, out: &mut Vec<Definition>) -> ParserResult<()> {
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
}
