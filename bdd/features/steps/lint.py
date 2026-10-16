import json
from pathlib import Path
import subprocess
import tempfile

from behave import given, when, then


_compiler = None


def compiler():
    global _compiler
    if _compiler is None:
        result = subprocess.run(
            [
                "cargo",
                "build",
                "-p",
                "xidlc",
                "--features",
                "cli,fmt",
                "--message-format=json",
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        for line in result.stdout.splitlines():
            if not line.startswith("{"):
                continue
            artifact = json.loads(line)
            if artifact.get("reason") == "compiler-artifact" and artifact.get(
                "executable"
            ):
                if artifact["target"]["name"] == "xidlc":
                    _compiler = artifact["executable"]
        assert _compiler, result.stdout + result.stderr
    return _compiler


@given("a versioned contract with annotation mistakes")
def mistaken_contract(context):
    context.temp_dir = tempfile.mkdtemp(prefix="xidl-lint-")
    context.lint_source = Path(context.temp_dir) / "api.idl"
    context.lint_source.write_text(context.text, encoding="utf-8")
    context.lint_original = context.lint_source.read_bytes()


@when("I check the contract with xidlc lint")
def lint_contract(context):
    context.lint_result = subprocess.run(
        [compiler(), "lint", str(context.lint_source)], capture_output=True, text=True
    )


@then("lint reports the annotation mistakes without rewriting the contract")
def diagnostics(context):
    result = context.lint_result
    assert result.returncode == 1, result.stderr
    assert result.stdout == ""
    for expected in [
        "error[misspelled-annotation]",
        "error[misplaced-annotation]",
        "did you mean `@removed`?",
        "did you mean `@get`?",
    ]:
        assert expected in result.stderr, result.stderr
    assert context.lint_source.read_bytes() == context.lint_original


def documents(context, folder):
    output = Path(context.temp_dir) / folder
    result = subprocess.run(
        [
            compiler(),
            "gen",
            "--out-dir",
            str(output),
            "openapi",
            str(context.lint_source),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    return {path.name: path.read_bytes() for path in output.glob("*.json")}


@then(
    "generation remains independent and correcting the typo changes the intended version"
)
def generation_and_correction(context):
    before = documents(context, "before")
    assert len(before) == 2
    for name, document in before.items():
        version = name.split(".")[-2]
        assert (
            "legacy"
            in json.loads(document)["components"]["schemas"][f"{version}.Item"][
                "properties"
            ]
        )
    source = context.lint_source.read_text()
    context.lint_source.write_text(
        source.replace("@remvoed", "@removed")
        .replace("@gett", "@get")
        .replace("@optional interface", "interface")
    )
    result = subprocess.run(
        [compiler(), "lint", str(context.lint_source)], capture_output=True, text=True
    )
    assert result.returncode == 0, result.stderr
    after = documents(context, "after")
    assert after == documents(context, "repeated"), "generation must be byte-identical"
    for name, document in after.items():
        version = name.split(".")[-2]
        properties = json.loads(document)["components"]["schemas"][f"{version}.Item"][
            "properties"
        ]
        assert ("legacy" in properties) == (".v1." in name), (name, properties)


@then("plugin warnings obey the CI policy across multiple files")
def warning_policy(context):
    directory = Path(context.temp_dir)
    plugin = directory / "plugin.idl"
    plugin.write_text("//@photon_audit\nstruct Audit { string id; };\n")
    result = subprocess.run(
        [compiler(), "lint", str(plugin)], capture_output=True, text=True
    )
    assert result.returncode == 0, result.stderr
    assert result.stderr.count("warning[unknown-annotation]") == 1
    denied = subprocess.run(
        [compiler(), "lint", "--deny", "warnings", str(plugin)],
        capture_output=True,
        text=True,
    )
    assert denied.returncode == 1
    assert denied.stderr == result.stderr, (
        "warning policy must not change diagnostic facts"
    )
    missing = directory / "missing.idl"
    batch = subprocess.run(
        [compiler(), "lint", str(missing), str(plugin), str(context.lint_source)],
        capture_output=True,
        text=True,
    )
    assert batch.returncode == 1
    assert str(missing) in batch.stderr
    assert "warning[unknown-annotation]" in batch.stderr
    assert "error[misspelled-annotation]" in batch.stderr
    assert sorted(path.name for path in directory.iterdir()) == [
        "api.idl",
        "plugin.idl",
    ]
