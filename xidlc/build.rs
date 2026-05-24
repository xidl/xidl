use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=XIDLC_GIT_HASH");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    if let Some(git_dir) = git_dir(&manifest_dir) {
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join("packed-refs").display()
        );
        println!("cargo:rerun-if-changed={}", git_dir.join("refs").display());
    }

    let git_hash = env::var("XIDLC_GIT_HASH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_stdout(&manifest_dir, &["rev-parse", "--short", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=XIDLC_GIT_HASH={git_hash}");
}

fn git_stdout(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(manifest_dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn git_dir(manifest_dir: &Path) -> Option<PathBuf> {
    let value = git_stdout(manifest_dir, &["rev-parse", "--git-dir"])?;
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(manifest_dir.join(path))
    }
}
