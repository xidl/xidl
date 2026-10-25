"""Compile the actual HTTP artifacts, including independently emitted adapters."""

import os
from pathlib import Path
import subprocess

from behave import then


@then("the emitted Rust library passes strict clippy")
def emitted_rust_clippy(context):
    root = Path.cwd()
    directory = Path(context.lang_dir)
    files = sorted(directory.glob("*.rs"))
    assert files, "Rust generation produced no library artifacts"
    source = directory / "src"
    source.mkdir()
    includes = "\n".join(f'include!("../{path.name}");' for path in files)
    (source / "lib.rs").write_text("pub mod generated {\n" + includes + "\n}\n")
    manifest = (
        root / "bdd/boilerplate/required_parameters/rust/Cargo.toml"
    ).read_text()
    (directory / "Cargo.toml").write_text(
        manifest.replace(
            "{{RUST_XIDL_RUST_AXUM_PATH}}", str(root / "crates/xidl-rust-axum")
        )
    )
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(root / "bdd/.temp/rust_target")
    result = subprocess.run(
        ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
        cwd=directory,
        env=env,
        capture_output=True,
        text=True,
        timeout=600,
    )
    assert result.returncode == 0, result.stdout + result.stderr
