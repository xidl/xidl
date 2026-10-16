use std::path::PathBuf;

/// Environment variable that overrides the default `~/.xidl` cache home.
pub const HOME_ENV: &str = "XIDL_HOME";

/// The shared cache home that holds downloaded packages and toolchains.
pub struct CacheHome;

impl CacheHome {
    /// `$XIDL_HOME`, or `~/.xidl`, when a home directory is known.
    pub fn resolve() -> Option<PathBuf> {
        if let Some(home) = std::env::var_os(HOME_ENV) {
            if !home.is_empty() {
                return Some(PathBuf::from(home));
            }
        }
        ["HOME", "USERPROFILE"]
            .iter()
            .find_map(std::env::var_os)
            .filter(|home| !home.is_empty())
            .map(|home| PathBuf::from(home).join(".xidl"))
    }

    /// Directory of cache `kind`, such as `pkg` or `toolchain`.
    pub fn dir(kind: &str) -> Option<PathBuf> {
        Self::resolve().map(|home| home.join(kind))
    }
}
