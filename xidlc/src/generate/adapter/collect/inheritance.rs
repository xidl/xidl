//! Preserve declaring scopes and logical base paths on flattened adapter fields.

use super::*;
use crate::generate::utils::scope::resolve_canonical;
use hir::VisitScopedNames;
use std::collections::BTreeSet;

impl SurfaceCollector {
    pub(super) fn inherit(&mut self, version: &str) -> IdlcResult<()> {
        let mut complete = BTreeSet::new();
        for key in self.parents.keys().cloned().collect::<Vec<_>>() {
            self.inherit_type(&key, version, &mut complete)?;
        }
        Ok(())
    }

    fn inherit_type(
        &mut self,
        key: &str,
        version: &str,
        complete: &mut BTreeSet<String>,
    ) -> IdlcResult<()> {
        if complete.contains(key) {
            return Ok(());
        }
        let Some((scope, mut parent)) = self.parents.get(key).cloned() else {
            return Ok(());
        };
        parent.remove_root_namespace(version);
        let parent = resolve_canonical(&scope, &parent, |name| self.types.contains_key(name))
            .ok_or_else(|| {
                IdlcError::rpc(format!(
                    "version adapters {key}: cannot resolve base {}",
                    parent.name.join("::")
                ))
            })?;
        self.inherit_type(&parent, version, complete)?;
        let Some(TypeShape::Struct(members)) = self.types.get(&parent) else {
            return Err(IdlcError::rpc(format!(
                "version adapters {key}: base {parent} is not a struct"
            )));
        };
        let mut inherited = members.clone();
        for member in &mut inherited {
            member.path.insert(0, parent.clone());
        }
        let Some(TypeShape::Struct(members)) = self.types.get_mut(key) else {
            return Err(IdlcError::rpc(format!(
                "version adapters {key}: derived declaration is not a struct"
            )));
        };
        inherited.append(members);
        *members = inherited;
        complete.insert(key.to_string());
        Ok(())
    }
}
