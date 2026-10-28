//! Typedef alias resolution for scalar transport positions.

use std::collections::HashMap;
use xidl_parser::hir;

/// Typedef declarators reachable from scalar transport positions. Text
/// positions decode raw wire strings, so a declared alias must expand to its
/// underlying shape before a coerce schema can be rendered for it.
pub(crate) struct TextAliases {
    entries: HashMap<Vec<String>, AliasEntry>,
    enums: std::collections::HashSet<Vec<String>>,
}

struct AliasEntry {
    /// Module the typedef was declared in; relative refs inside its shape
    /// resolve against this path.
    defining_path: Vec<String>,
    ty: hir::TypeSpec,
}

impl TextAliases {
    pub(crate) fn collect(spec: &hir::Specification) -> Self {
        let mut entries = HashMap::new();
        let mut enums = std::collections::HashSet::new();
        Self::collect_defs(&spec.0, &[], &mut entries, &mut enums);
        Self { entries, enums }
    }

    fn collect_defs(
        defs: &[hir::Definition],
        module_path: &[String],
        entries: &mut HashMap<Vec<String>, AliasEntry>,
        enums: &mut std::collections::HashSet<Vec<String>>,
    ) {
        for def in defs {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    let mut path = module_path.to_vec();
                    path.push(module.ident.clone());
                    Self::collect_defs(&module.definition, &path, entries, enums);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::ConstrTypeDcl(
                    hir::ConstrTypeDcl::EnumDcl(enumeration),
                ))
                | hir::Definition::ConstrTypeDcl(hir::ConstrTypeDcl::EnumDcl(enumeration)) => {
                    let mut path = module_path.to_vec();
                    path.push(enumeration.ident.clone());
                    enums.insert(path);
                }
                hir::Definition::TypeDcl(hir::TypeDcl::TypedefDcl(typedef)) => {
                    let hir::TypedefType::TypeSpec(ty) = &typedef.ty else {
                        continue;
                    };
                    for decl in &typedef.decl {
                        let mut key = module_path.to_vec();
                        key.push(
                            crate::generate::typescript::definition::names::declarator_name(decl)
                                .to_string(),
                        );
                        entries.insert(
                            key,
                            AliasEntry {
                                defining_path: module_path.to_vec(),
                                ty: ty.clone(),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Resolves `ScopedName` chains down to the underlying shape, expanding
    /// sequence elements too. Constructor types end the expansion.
    pub(crate) fn expand(&self, ty: &hir::TypeSpec, module_path: &[String]) -> hir::TypeSpec {
        self.expand_inner(
            ty.clone(),
            module_path,
            &mut std::collections::BTreeSet::new(),
        )
    }

    /// Text-backed members decode by their declaration, never by their contents.
    pub(crate) fn is_text(&self, ty: &hir::TypeSpec, module_path: &[String]) -> bool {
        match self.expand(ty, module_path) {
            hir::TypeSpec::StringType(_) | hir::TypeSpec::WideStringType(_) => true,
            hir::TypeSpec::ScopedName(name) => self
                .resolve(&name, module_path)
                .is_some_and(|path| self.enums.contains(&path)),
            _ => false,
        }
    }

    fn expand_inner(
        &self,
        ty: hir::TypeSpec,
        module_path: &[String],
        visited: &mut std::collections::BTreeSet<Vec<String>>,
    ) -> hir::TypeSpec {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let Some(path) = self.resolve(&name, module_path) else {
                    return hir::TypeSpec::ScopedName(name);
                };
                let Some(AliasEntry { defining_path, ty }) = self.entries.get(&path) else {
                    return hir::TypeSpec::ScopedName(hir::ScopedName {
                        name: path,
                        is_root: true,
                    });
                };
                if !visited.insert(path) {
                    return hir::TypeSpec::ScopedName(name);
                }
                self.expand_inner(ty.clone(), defining_path.as_slice(), visited)
            }
            hir::TypeSpec::SequenceType(mut sequence) => {
                sequence.ty =
                    Box::new(self.expand_inner((*sequence.ty).clone(), module_path, visited));
                hir::TypeSpec::SequenceType(sequence)
            }
            resolved => resolved,
        }
    }

    fn resolve(&self, name: &hir::ScopedName, module_path: &[String]) -> Option<Vec<String>> {
        crate::generate::utils::scope::resolve_canonical(module_path, name, |key| {
            let path = key.split("::").map(str::to_owned).collect::<Vec<_>>();
            self.entries.contains_key(&path) || self.enums.contains(&path)
        })
        .map(|key| key.split("::").map(str::to_owned).collect())
    }
}
