use xidl_parser::hir;

pub(super) struct ZodImportCollector;

impl ZodImportCollector {
    pub(super) fn collect_exported_names(spec: &hir::Specification) -> Vec<String> {
        let mut names = std::collections::BTreeSet::new();
        Self::collect_defs(&spec.0, &mut names);
        names.into_iter().collect()
    }

    pub(super) fn is_word_in_text(word: &str, text: &str) -> bool {
        let mut start = 0;
        while let Some(idx) = text[start..].find(word) {
            let match_start = start + idx;
            let match_end = match_start + word.len();
            let before_ok = match_start == 0 || {
                text[..match_start]
                    .chars()
                    .next_back()
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true)
            };
            let after_ok = match_end == text.len() || {
                text[match_end..]
                    .chars()
                    .next()
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true)
            };
            if before_ok && after_ok {
                return true;
            }
            start = match_start + 1;
        }
        false
    }

    fn collect_defs(defs: &[hir::Definition], names: &mut std::collections::BTreeSet<String>) {
        for def in defs {
            match def {
                hir::Definition::ModuleDcl(module) => {
                    Self::collect_defs(&module.definition, names);
                }
                hir::Definition::ConstrTypeDcl(constr) => {
                    Self::collect_constr_names(constr, names);
                }
                hir::Definition::TypeDcl(ty) => match ty {
                    hir::TypeDcl::ConstrTypeDcl(constr) => {
                        Self::collect_constr_names(constr, names);
                    }
                    hir::TypeDcl::TypedefDcl(typedef) => {
                        for decl in &typedef.decl {
                            let name = crate::generate::typescript::definition::names::ts_ident(
                                crate::generate::typescript::definition::names::declarator_name(
                                    decl,
                                ),
                            );
                            names.insert(name);
                        }
                    }
                    hir::TypeDcl::NativeDcl(native) => {
                        let name = crate::generate::typescript::definition::names::ts_ident(
                            &native.decl.0,
                        );
                        names.insert(name);
                    }
                },
                hir::Definition::ExceptDcl(except) => {
                    let name =
                        crate::generate::typescript::definition::names::ts_ident(&except.ident);
                    names.insert(name);
                }
                _ => {}
            }
        }
    }

    fn collect_constr_names(
        constr: &hir::ConstrTypeDcl,
        names: &mut std::collections::BTreeSet<String>,
    ) {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = crate::generate::typescript::definition::names::ts_ident(&def.ident);
                names.insert(name);
            }
            _ => {}
        }
    }
}
