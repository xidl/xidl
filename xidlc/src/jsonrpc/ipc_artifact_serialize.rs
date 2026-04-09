use super::{Artifact, ArtifactFile, ArtifactHir, ArtifactHttpHir, ArtifactKind};

impl serde::Serialize for Artifact {
    fn serialize<__S>(&self, __serializer: __S) -> serde::__private228::Result<__S::Ok, __S::Error>
    where
        __S: serde::Serializer,
    {
        match self.tag {
            ArtifactKind::Hir => {
                let mut s = serde::Serializer::serialize_struct_variant(
                    __serializer,
                    "Artifact",
                    0,
                    "ArtifactKind::Hir",
                    size_of::<ArtifactHir>(),
                )?;
                let x = unsafe { std::ops::Deref::deref(&self.data.hir) };
                serde::ser::SerializeStructVariant::serialize_field(
                    &mut s,
                    "ArtifactKind::Hir",
                    x,
                )?;
                serde::ser::SerializeStructVariant::end(s)
            }
            ArtifactKind::HttpHir => {
                let mut s = serde::Serializer::serialize_struct_variant(
                    __serializer,
                    "Artifact",
                    1,
                    "ArtifactKind::HttpHir",
                    size_of::<ArtifactHttpHir>(),
                )?;
                let x = unsafe { std::ops::Deref::deref(&self.data.http_hir) };
                serde::ser::SerializeStructVariant::serialize_field(
                    &mut s,
                    "ArtifactKind::HttpHir",
                    x,
                )?;
                serde::ser::SerializeStructVariant::end(s)
            }
            ArtifactKind::File => {
                let mut s = serde::Serializer::serialize_struct_variant(
                    __serializer,
                    "Artifact",
                    2,
                    "ArtifactKind::File",
                    size_of::<ArtifactFile>(),
                )?;
                let x = unsafe { std::ops::Deref::deref(&self.data.file) };
                serde::ser::SerializeStructVariant::serialize_field(
                    &mut s,
                    "ArtifactKind::File",
                    x,
                )?;
                serde::ser::SerializeStructVariant::end(s)
            }
        }
    }
}
