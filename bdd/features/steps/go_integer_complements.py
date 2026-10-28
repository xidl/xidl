"""Compile and execute Go masks from included and imported IDL constants."""

import os
from pathlib import Path
import shutil
import subprocess

from behave import then, when

from version_adapters import run


@when("I generate the split Go integer mask library")
def generate_go_masks(context):
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
            *map(str, source.parent.rglob("*.idl")),
        ],
        cwd=root,
    )
    command = [str(cli), "gen", "--out-dir", str(output), "go", str(source)]
    run(command, cwd=root)
    first = {
        str(path.relative_to(output)): path.read_bytes()
        for path in output.rglob("*.go")
    }
    assert set(first) == {"api.go", "masks/dep.go"}, first.keys()
    run(command, cwd=root)
    assert first == {
        str(path.relative_to(output)): path.read_bytes()
        for path in output.rglob("*.go")
    }


@then("the compiled Go library preserves mask bounds and permission decisions")
def execute_go_masks(context):
    output = Path(context.lang_dir)
    boilerplate = Path.cwd() / "bdd/boilerplate/go_integer_complements"
    for name in ("go.mod", "masks_test.go"):
        shutil.copy(boilerplate / name, output / name)
    shutil.copy(boilerplate / "masks.go.mod", output / "masks/go.mod")
    env = os.environ.copy()
    env.update(GOPROXY="off", GOSUMDB="off", GOTOOLCHAIN="local", GOWORK="off")
    result = subprocess.run(
        ["go", "test", "-count=1", "./..."],
        cwd=output,
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert result.returncode == 0, result.stdout + result.stderr
