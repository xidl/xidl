use std::marker::PhantomData;

use super::{Artifact, ArtifactData, ArtifactKind};

impl<'de> serde::Deserialize<'de> for Artifact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &[
            "ArtifactKind::Hir",
            "ArtifactKind::HttpHir",
            "ArtifactKind::File",
        ];

        enum Variant {
            Hir,
            HttpHir,
            File,
        }

        impl<'de> serde::Deserialize<'de> for Variant {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct VariantVisitor;

                impl serde::de::Visitor<'_> for VariantVisitor {
                    type Value = Variant;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                        formatter.write_str("union variant")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "ArtifactKind::Hir" => Ok(Variant::Hir),
                            "ArtifactKind::HttpHir" => Ok(Variant::HttpHir),
                            "ArtifactKind::File" => Ok(Variant::File),
                            _ => Err(E::unknown_variant(value, VARIANTS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(VariantVisitor)
            }
        }

        struct ArtifactVisitor;

        impl<'de> serde::de::Visitor<'de> for ArtifactVisitor {
            type Value = Artifact;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("externally tagged union Artifact")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                let (variant, access) = data.variant::<Variant>()?;
                match variant {
                    Variant::Hir => deserialize_case(
                        access,
                        "ArtifactKind::Hir",
                        &["ArtifactKind::Hir"],
                        ArtifactKind::Hir,
                        |data| ArtifactData {
                            hir: core::mem::ManuallyDrop::new(data),
                        },
                    ),
                    Variant::HttpHir => deserialize_case(
                        access,
                        "ArtifactKind::HttpHir",
                        &["ArtifactKind::HttpHir"],
                        ArtifactKind::HttpHir,
                        |data| ArtifactData {
                            http_hir: core::mem::ManuallyDrop::new(data),
                        },
                    ),
                    Variant::File => deserialize_case(
                        access,
                        "ArtifactKind::File",
                        &["ArtifactKind::File"],
                        ArtifactKind::File,
                        |data| ArtifactData {
                            file: core::mem::ManuallyDrop::new(data),
                        },
                    ),
                }
            }
        }

        deserializer.deserialize_enum("Artifact", VARIANTS, ArtifactVisitor)
    }
}

fn deserialize_case<'de, A, T, F>(
    access: A,
    field: &'static str,
    fields: &'static [&'static str],
    tag: ArtifactKind,
    wrap: F,
) -> Result<Artifact, A::Error>
where
    A: serde::de::VariantAccess<'de>,
    T: serde::Deserialize<'de>,
    F: FnOnce(T) -> ArtifactData,
{
    let value = serde::de::VariantAccess::struct_variant(
        access,
        fields,
        ArtifactFieldVisitor::<T> {
            field,
            marker: PhantomData,
        },
    )?;
    Ok(Artifact {
        tag,
        data: wrap(value),
    })
}

struct ArtifactFieldVisitor<T> {
    field: &'static str,
    marker: PhantomData<T>,
}

impl<'de, T> serde::de::Visitor<'de> for ArtifactFieldVisitor<T>
where
    T: serde::Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_fmt(format_args!("struct variant {}", self.field))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut value = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.field {
                if value.is_some() {
                    return Err(serde::de::Error::duplicate_field(self.field));
                }
                value = Some(map.next_value()?);
            } else {
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }

        value.ok_or_else(|| serde::de::Error::missing_field(self.field))
    }
}
