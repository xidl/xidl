use super::{Annotation, AnnotationParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SerializeKind {
    Cdr,
    PlainCdr,
    PlCdr,
    PlainCdr2,
    DelimitedCdr,
    PlCdr2,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SerializeVersion {
    Xcdr1,
    Xcdr2,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct SerializeConfig {
    pub explicit_kind: Option<SerializeKind>,
    pub version: Option<SerializeVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Extensibility {
    Final,
    Appendable,
    Mutable,
    None,
}

impl SerializeConfig {
    pub fn apply_pragma(&mut self, pragma: super::Pragma) {
        match pragma {
            super::Pragma::Custom(_) => {}
            super::Pragma::XidlcSerialize(kind) => self.explicit_kind = Some(kind),
            super::Pragma::XidlcVersion(version) => {
                self.version = Some(version);
                self.explicit_kind = None;
            }
            super::Pragma::XidlcPackage(_)
            | super::Pragma::XidlcOpenApiVersion(_)
            | super::Pragma::XidlcOpenApiService { .. } => {}
        }
    }

    pub fn resolve(&self, extensibility: Extensibility) -> SerializeKind {
        if let Some(kind) = self.explicit_kind {
            return kind;
        }

        match self.version {
            None => SerializeKind::Cdr,
            Some(SerializeVersion::Xcdr1) => match extensibility {
                Extensibility::Mutable => SerializeKind::PlCdr,
                Extensibility::Final | Extensibility::Appendable => SerializeKind::Cdr,
                Extensibility::None => SerializeKind::PlainCdr,
            },
            Some(SerializeVersion::Xcdr2) => match extensibility {
                Extensibility::Final => SerializeKind::PlainCdr2,
                Extensibility::Appendable => SerializeKind::DelimitedCdr,
                Extensibility::Mutable => SerializeKind::PlCdr2,
                Extensibility::None => SerializeKind::Cdr,
            },
        }
    }

    pub fn resolve_for_annotations(&self, annotations: &[Annotation]) -> SerializeKind {
        self.resolve(extensibility_from_annotations(annotations))
    }
}

pub fn extensibility_from_annotations(annotations: &[Annotation]) -> Extensibility {
    let mut final_flag = false;
    let mut appendable = false;
    let mut mutable = false;

    for annotation in annotations {
        if let Annotation::Builtin { name, .. } = annotation {
            if name.eq_ignore_ascii_case("final") {
                final_flag = true;
            } else if name.eq_ignore_ascii_case("appendable") {
                appendable = true;
            } else if name.eq_ignore_ascii_case("mutable") {
                mutable = true;
            }
        }

        if let Annotation::Builtin { name, params } = annotation {
            if name.eq_ignore_ascii_case("extensibility") {
                if let Some(AnnotationParams::Raw(raw)) = params {
                    let value = raw.trim().trim_matches('"');
                    if value.eq_ignore_ascii_case("final") {
                        final_flag = true;
                    } else if value.eq_ignore_ascii_case("appendable") {
                        appendable = true;
                    } else if value.eq_ignore_ascii_case("mutable") {
                        mutable = true;
                    }
                }
            }
        }
    }

    if mutable {
        Extensibility::Mutable
    } else if appendable {
        Extensibility::Appendable
    } else if final_flag {
        Extensibility::Final
    } else {
        Extensibility::None
    }
}

#[cfg(test)]
mod tests;
