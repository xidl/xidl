//! Validation of `@replaces` links against the timeline and the siblings.
//!
//! The annotation names the earlier member a declaration takes over from.
//! Both declarations stay as written; this pass only proves the link is
//! mechanically usable, so the version adapters can copy the value at the
//! retirement step without a hook and without a storage reader.

use super::super::VersioningError;
use super::super::paths::{declarator_name, join};
use super::super::timeline::VersionTimeline;
use super::availability::{render_reference, resolve_reference};
use crate::error::ParserResult;
use crate::hir::{
    Annotation, ConstrTypeDcl, Definition, Export, InterfaceDclInner, Member, TypeDcl,
};

/// Proves every `@replaces` link of a lowered specification.
pub(crate) fn validate(definitions: &[Definition], timeline: &VersionTimeline) -> ParserResult<()> {
    walk(definitions, "", timeline)
}

fn walk(definitions: &[Definition], path: &str, timeline: &VersionTimeline) -> ParserResult<()> {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                let here = join(path, &module.ident);
                walk(&module.definition, &here, timeline)?;
            }
            Definition::ConstrTypeDcl(constr) => constr_type(constr, path, timeline)?,
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                constr_type(constr, path, timeline)?;
            }
            Definition::ExceptDcl(except) => {
                let here = join(path, &except.ident);
                members(&except.member, &here, timeline)?;
            }
            Definition::InterfaceDcl(interface) => {
                let InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                    continue;
                };
                let here = join(path, &def.header.ident);
                let Some(body) = &def.interface_body else {
                    continue;
                };
                for export in &body.0 {
                    match export {
                        Export::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                            constr_type(constr, &here, timeline)?;
                        }
                        Export::ExceptDcl(except) => {
                            let here = join(&here, &except.ident);
                            members(&except.member, &here, timeline)?;
                        }
                        _ => {}
                    }
                }
            }
            Definition::Pragma(_) | Definition::ConstDcl(_) | Definition::TypeDcl(_) => {}
        }
    }
    Ok(())
}

fn constr_type(constr: &ConstrTypeDcl, path: &str, timeline: &VersionTimeline) -> ParserResult<()> {
    if let ConstrTypeDcl::StructDcl(dcl) = constr {
        let here = join(path, &dcl.ident);
        members(&dcl.member, &here, timeline)?;
    }
    Ok(())
}

fn members(members: &[Member], path: &str, timeline: &VersionTimeline) -> ParserResult<()> {
    let mut replaced = std::collections::BTreeSet::new();
    for member in members {
        let Some(name) = replaces_name(&member.annotations) else {
            continue;
        };
        let decl = join(path, &declarators_name(member));
        let invalid = |reason: String| VersioningError::InvalidReplaces {
            decl: decl.clone(),
            reason,
        };
        let [declarator] = member.ident.as_slice() else {
            return Err(invalid("requires a single declarator".to_string()).into());
        };
        let current = declarator_name(declarator);
        if name == current {
            return Err(invalid("replaces itself".to_string()).into());
        }
        if !replaced.insert(name.clone()) {
            return Err(invalid(format!("names {name} more than once")).into());
        }
        let Some(earlier) = members.iter().find(|sibling| {
            sibling
                .ident
                .iter()
                .any(|decl| declarator_name(decl) == name)
        }) else {
            return Err(invalid(format!(
                "names {name}, but no member with that name is declared here"
            ))
            .into());
        };
        let retirement = earlier
            .annotations
            .iter()
            .find_map(|annotation| match annotation {
                Annotation::Removed { version } => Some(version.clone()),
                _ => None,
            });
        let Some(retirement) = retirement else {
            return Err(invalid(format!(
                "names {name}, which is not removed; a replaced member carries @removed"
            ))
            .into());
        };
        let at = resolve_reference(timeline, "@replaces", &decl, &retirement)?.index();
        let added = member
            .annotations
            .iter()
            .find_map(|annotation| match annotation {
                Annotation::Added { version } => Some(version.clone()),
                _ => None,
            });
        if let Some(added) = &added {
            let introduction = resolve_reference(timeline, "@added", &decl, added)?.index();
            if introduction > at {
                return Err(invalid(format!(
                    "is added at {}, after {name} retires at {}; the replaced member must still be declared in that version",
                    render_reference(added),
                    render_reference(&retirement),
                ))
                .into());
            }
        }
        let own_retirement = member
            .annotations
            .iter()
            .find_map(|annotation| match annotation {
                Annotation::Removed { version } => Some(version.clone()),
                _ => None,
            });
        if let Some(own) = &own_retirement {
            let until = resolve_reference(timeline, "@removed", &decl, own)?.index();
            if until <= at {
                return Err(invalid(format!(
                    "itself retires at {}, before {name} does at {}",
                    render_reference(own),
                    render_reference(&retirement),
                ))
                .into());
            }
        }
    }
    Ok(())
}

/// The `@replaces` earlier name carried by these annotations, if any.
pub(crate) fn replaces_name(annotations: &[Annotation]) -> Option<String> {
    annotations.iter().find_map(|annotation| match annotation {
        Annotation::Replaces { name } => Some(name.clone()),
        _ => None,
    })
}

fn declarators_name(member: &Member) -> String {
    member
        .ident
        .iter()
        .map(declarator_name)
        .collect::<Vec<_>>()
        .join(", ")
}
