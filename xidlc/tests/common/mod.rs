//! Shared harness for the `xidlc version` subcommand end-to-end tests.

use clap::Parser as _;
use std::fs;
use std::path::PathBuf;
use std::process;
use xidlc::cli::Cli;

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

    pub fn run(&self, args: &[&str]) -> Result<(), String> {
        let cli = Cli::try_parse_from(args).expect("parse version args");
        cli.run().map_err(|error| error.to_string())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
