//! Shared harness for the `xidlc version` subcommand end-to-end tests.

use std::fs;
use std::path::PathBuf;
use std::process;

pub struct Workspace {
    pub dir: PathBuf,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("xidl-version-{}-{}", process::id(), name));
        fs::create_dir_all(&dir).expect("create workspace");
        Self { dir }
    }

    pub fn contract_path(&self, name: &str, source: &str) -> PathBuf {
        let path = self.dir.join(format!("{name}.idl"));
        fs::write(&path, source).expect("write contract");
        path
    }

    /// Runs the built `xidlc` binary inside the workspace — the exact process
    /// a release gate would run. `args` includes the program name. `Ok`
    /// carries stdout on exit success; `Err` carries stderr on failure.
    pub fn run(&self, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_xidlc"))
            .args(&args[1..])
            .current_dir(&self.dir)
            .output()
            .expect("run the xidlc binary");
        if output.status.success() {
            Ok(String::from_utf8(output.stdout).expect("stdout is utf-8"))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
