#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct ArtifactHir {
    pub lang: String,
    pub hir: ::xidl_parser::hir::Specification,
    pub props: ::xidl_parser::hir::ParserProperties,
}

impl ArtifactHir {}

#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct ArtifactHttpHir {
    pub lang: String,
    pub hir: ::xidl_parser::hir::Specification,
    pub http_hir: crate::generate::http_hir::HttpHirDocument,
    pub props: ::xidl_parser::hir::ParserProperties,
}

impl ArtifactHttpHir {}

#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct ArtifactFile {
    pub path: String,
    pub content: String,
}

impl ArtifactFile {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
pub enum ArtifactKind {
    Hir,
    HttpHir,
    File,
}

pub struct Artifact {
    pub(super) tag: ArtifactKind,
    pub(super) data: ArtifactData,
}

pub(super) union ArtifactData {
    pub(super) hir: core::mem::ManuallyDrop<ArtifactHir>,
    pub(super) http_hir: core::mem::ManuallyDrop<ArtifactHttpHir>,
    pub(super) file: core::mem::ManuallyDrop<ArtifactFile>,
}

impl Drop for Artifact {
    fn drop(&mut self) {
        match self.tag {
            ArtifactKind::Hir => unsafe {
                core::mem::ManuallyDrop::drop(&mut self.data.hir);
            },
            ArtifactKind::HttpHir => unsafe {
                core::mem::ManuallyDrop::drop(&mut self.data.http_hir);
            },
            ArtifactKind::File => unsafe {
                core::mem::ManuallyDrop::drop(&mut self.data.file);
            },
        }
    }
}

impl Artifact {
    pub fn new_hir(value: ArtifactHir) -> Self {
        Self {
            tag: ArtifactKind::Hir,
            data: ArtifactData {
                hir: core::mem::ManuallyDrop::new(value),
            },
        }
    }

    pub fn is_hir(&self) -> bool {
        matches!(self.tag, ArtifactKind::Hir)
    }

    pub fn as_hir(&self) -> &ArtifactHir {
        debug_assert!(self.is_hir());
        unsafe { &self.data.hir }
    }

    pub fn as_hir_mut(&mut self) -> &mut ArtifactHir {
        debug_assert!(self.is_hir());
        unsafe { &mut self.data.hir }
    }

    pub fn into_hir(self) -> ArtifactHir {
        debug_assert!(self.is_hir());
        unsafe {
            let mut forget = core::mem::ManuallyDrop::new(self);
            core::mem::ManuallyDrop::take(&mut forget.data.hir)
        }
    }

    pub fn new_http_hir(value: ArtifactHttpHir) -> Self {
        Self {
            tag: ArtifactKind::HttpHir,
            data: ArtifactData {
                http_hir: core::mem::ManuallyDrop::new(value),
            },
        }
    }

    pub fn is_http_hir(&self) -> bool {
        matches!(self.tag, ArtifactKind::HttpHir)
    }

    pub fn as_http_hir(&self) -> &ArtifactHttpHir {
        debug_assert!(self.is_http_hir());
        unsafe { &self.data.http_hir }
    }

    pub fn as_http_hir_mut(&mut self) -> &mut ArtifactHttpHir {
        debug_assert!(self.is_http_hir());
        unsafe { &mut self.data.http_hir }
    }

    pub fn into_http_hir(self) -> ArtifactHttpHir {
        debug_assert!(self.is_http_hir());
        unsafe {
            let mut forget = core::mem::ManuallyDrop::new(self);
            core::mem::ManuallyDrop::take(&mut forget.data.http_hir)
        }
    }

    pub fn new_file(value: ArtifactFile) -> Self {
        Self {
            tag: ArtifactKind::File,
            data: ArtifactData {
                file: core::mem::ManuallyDrop::new(value),
            },
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self.tag, ArtifactKind::File)
    }

    pub fn as_file(&self) -> &ArtifactFile {
        debug_assert!(self.is_file());
        unsafe { &self.data.file }
    }

    pub fn as_file_mut(&mut self) -> &mut ArtifactFile {
        debug_assert!(self.is_file());
        unsafe { &mut self.data.file }
    }

    pub fn into_file(self) -> ArtifactFile {
        debug_assert!(self.is_file());
        unsafe {
            let mut forget = core::mem::ManuallyDrop::new(self);
            core::mem::ManuallyDrop::take(&mut forget.data.file)
        }
    }

    pub fn tag(&self) -> &ArtifactKind {
        &self.tag
    }
}
