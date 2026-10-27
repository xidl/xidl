"""Execute constants from independently generated Rust compilation units."""

import os
from pathlib import Path
import shutil
import subprocess

from behave import then, when

from version_adapters import run


@when("I generate the split integer constant Rust library")
def generate_constants(context):
    root = Path.cwd()
    cli = root / "target/debug/xidlc"
    source = Path(context.idl_file)
    output = Path(context.lang_dir)
    run(
        [
            str(cli),
            "lint",
            "--deny",
            "warnings",
            *map(str, source.parent.glob("*.idl")),
        ],
        cwd=root,
    )
    command = [str(cli), "gen", "--out-dir", str(output), "rust", str(source)]
    run(command, cwd=root)
    first = {path.name: path.read_bytes() for path in output.glob("*.rs")}
    assert set(first) == {"api.rs", "core.rs"}, first.keys()
    run(command, cwd=root)
    assert first == {path.name: path.read_bytes() for path in output.glob("*.rs")}


@then("the compiled constant library preserves masks and imported values")
def execute_constants(context):
    root = Path.cwd()
    output = Path(context.lang_dir)
    boilerplate = root / "bdd/boilerplate/integer_constants"
    for name in ("Cargo.toml", "main.rs"):
        shutil.copy(boilerplate / name, output / name)
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(root / "bdd/.temp/rust_target")
    # The generated units have only serde dependencies, already used by this suite.
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--offline"],
        cwd=output,
        env=env,
        capture_output=True,
        text=True,
        timeout=300,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout.strip() == "PASS: integer constants"
