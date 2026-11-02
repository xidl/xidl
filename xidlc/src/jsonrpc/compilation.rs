//! Compilation input for generators that assemble shared artifacts.

use super::CodegenInput;
use serde::{Deserialize, Serialize};
use xidl_parser::versioning::map::VersionMap;

/// Supported compilation input format.
pub const COMPILATION_FORMAT: u32 = 1;

/// Complete compilation graph passed to a compilation-scoped generator once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationInput {
    /// Version of this compilation envelope, independent of version-map formats.
    pub format: u32,
    /// Compilation units, with their selected semantic inputs.
    pub units: Vec<CompilationUnit>,
}

/// One compilation unit with logical identity and compiler-owned version facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationUnit {
    /// Import prefix, unique within this compilation graph.
    pub id: String,
    /// Whether the user explicitly requested this unit as a generation root.
    pub is_root: bool,
    /// Resolved import aliases, without absolute machine paths.
    pub imports: Vec<UnitImport>,
    /// Adjacent version facts when the unit declares a timeline.
    pub version_map: Option<VersionMap>,
    /// Selected version inputs, or one unversioned semantic input.
    pub inputs: Vec<VersionInput>,
}

/// One resolved import's logical identity and optional version binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitImport {
    /// Prefix used by named references in the importing unit.
    pub prefix: String,
    /// Version binding, when present in the import declaration.
    pub version: Option<String>,
    /// Version member names declared by the imported compilation unit.
    pub versions: Vec<String>,
}

/// A typed semantic input and its selected version identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInput {
    /// Source enum member name, absent for an unversioned contract.
    pub version: Option<String>,
    /// Existing typed transport or language-neutral semantic input.
    pub input: CodegenInput,
}
