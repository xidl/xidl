use enum_dispatch::enum_dispatch;

use super::File;
use crate::error::IdlcResult;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[enum_dispatch]
pub trait OutputTargetTrait {
    fn write_files(&self, files: Vec<File>) -> IdlcResult<()>;
}

#[enum_dispatch(OutputTargetTrait)]
pub enum OutputTarget {
    Dummy(DummyOutputTarget),
    Real(RealOutputTarget),
}

impl OutputTarget {
    pub fn new_dummy() -> Self {
        Self::Dummy(DummyOutputTarget)
    }

    pub fn new_real(out_dir: &str) -> IdlcResult<Self> {
        Ok(Self::Real(RealOutputTarget::new(out_dir)?))
    }
}

pub struct DummyOutputTarget;

impl OutputTargetTrait for DummyOutputTarget {
    fn write_files(&self, _files: Vec<File>) -> IdlcResult<()> {
        Ok(())
    }
}

pub struct RealOutputTarget {
    out_dir: String,
}

impl RealOutputTarget {
    fn new(out_dir: &str) -> IdlcResult<Self> {
        if out_dir != "-" {
            fs::create_dir_all(out_dir).map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!("Failed to create output directory '{}': {}", out_dir, err),
                )
            })?;
        }
        Ok(Self {
            out_dir: out_dir.to_string(),
        })
    }
}
impl OutputTargetTrait for RealOutputTarget {
    fn write_files(&self, files: Vec<File>) -> IdlcResult<()> {
        let mut order = Vec::new();
        let mut merged: HashMap<String, String> = HashMap::new();
        for file in files {
            let File { path, content } = file;
            if let Some(existing) = merged.get_mut(&path) {
                existing.push_str(&content);
            } else {
                order.push(path.clone());
                merged.insert(path, content);
            }
        }

        let out_dir_path = Path::new(&self.out_dir);

        for path in order {
            let content = merged.remove(&path).unwrap_or_default();
            let file_path = Path::new(&path);
            let out_path = if file_path.is_absolute() {
                file_path.to_path_buf()
            } else {
                out_dir_path.join(file_path)
            };
            if self.out_dir != "-"
                && let Some(parent) = out_path.parent()
            {
                fs::create_dir_all(parent)?;
            }
            tracing::info!("write file: {}", out_path.display());
            if self.out_dir == "-" {
                println!("{}", content);
            } else {
                fs::write(out_path, content)?;
            }
        }
        Ok(())
    }
}
