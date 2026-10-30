"""Compile the same scoped models with ordinary Go and Go REST output."""

import os
from pathlib import Path
import shutil
import subprocess

from behave import then, when

from version_adapters import run


@when('I generate the scoped Go contract with the "{target}" target')
def generate_scoped_go(context, target):
    assert target in ("go", "go-rest"), target
    context.go_scope_target = target
    root = Path.cwd()
    cli = root / os.environ.get("CARGO_TARGET_DIR", "target") / "debug/xidlc"
    run([str(cli), "lint", "--deny", "warnings", context.idl_file], cwd=root)
    run(
        [
            str(cli),
            "gen",
            "-o",
            context.lang_dir,
            target,
            "--client",
            "--server",
            context.idl_file,
        ],
        cwd=root,
    )


@then("the compiled Go contract preserves module type identity")
def execute_scoped_go(context):
    output = Path(context.lang_dir)
    boilerplate = Path.cwd() / "bdd/boilerplate/go_scopes"
    files = ["models_test.go"]
    if context.go_scope_target == "go-rest":
        files.append("http_test.go")
    for name in files:
        shutil.copy(boilerplate / name, output / name)
    shutil.copy(boilerplate / "sources.go.mod", output / "sources/go.mod")
    module = (boilerplate / "go.mod").read_text()
    for key, path in [
        ("GOLANG_XIDL_GO_PATH", "golang/xidl-go"),
        ("GOLANG_XIDL_GO_CODEC_PATH", "golang/xidl-go-codec"),
        ("GOLANG_XIDL_GO_REST_PATH", "golang/xidl-go-rest"),
    ]:
        module = module.replace("{{" + key + "}}", str(Path.cwd() / path))
    (output / "go.mod").write_text(module)
    env = os.environ.copy()
    env.update(GOTOOLCHAIN="local", GOWORK="off")
    subprocess.run(["go", "mod", "tidy"], cwd=output, env=env, check=True, timeout=60)
    env.update(GOPROXY="off", GOSUMDB="off")
    result = subprocess.run(
        ["go", "test", "-count=1", "./..."],
        cwd=output,
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert result.returncode == 0, result.stdout + result.stderr
