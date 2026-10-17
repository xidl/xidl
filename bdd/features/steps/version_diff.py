"""Real CLI checks for canonical version diffs and their failure contract."""

import os
from pathlib import Path
import subprocess
import tempfile
import shutil

from behave import given, then, when


def diff_command(source, from_version, to_version):
    return [
        "cargo",
        "run",
        "-p",
        "xidlc",
        "--features",
        "cli,fmt",
        "--",
        "version",
        "diff",
        from_version,
        to_version,
        str(source),
    ]


def run_diff(context, source, from_version, to_version):
    result = subprocess.run(
        diff_command(source, from_version, to_version),
        cwd=Path.cwd(),
        capture_output=True,
        text=True,
        env={**os.environ, "XIDLC_VERSION": "0.0.1", "XIDLC_GIT_HASH": "snapshot"},
        timeout=180,
    )
    return result


@given('a version diff IDL file "{idl_file}"')
def version_diff_file(context, idl_file):
    context.version_diff_source = Path.cwd() / idl_file


@when('I compare version "{from_version}" to "{to_version}" with xidlc version diff')
def compare_versions(context, from_version, to_version):
    context.version_diff_result = run_diff(
        context, context.version_diff_source, from_version, to_version
    )


@then("the version diff exits with 1 and contains the contract changes")
def changed_diff(context):
    result = context.version_diff_result
    assert result.returncode == 1, result.stderr
    assert "--- version_diff.idl @ v1 (2026-04)" in result.stdout
    assert "+++ version_diff.idl @ v2 (2026-07)" in result.stdout
    assert "-    string text;" in result.stdout
    assert "+    string body;" in result.stdout
    assert "+    Message list(" in result.stdout
    assert "-    string export_csv();" in result.stdout
    assert "+        @query" in result.stdout


@then("the version diff exits with 0 and no output")
def unchanged_diff(context):
    result = context.version_diff_result
    assert result.returncode == 0, result.stderr
    assert result.stdout == "", result.stdout
    assert "version diff:" not in result.stderr, result.stderr


@when("I compare the original and reformatted version diff sources")
def compare_reformatted_sources(context):
    original = context.version_diff_source.read_text()
    reformatted = "// moved comment\n\n" + original.replace(
        "struct Message {", "\n\nstruct Message\n{"
    ).replace("@rust-derive(Debug)", "@rust-derive( /* derive */ Debug /* same */ )")
    temp_dir = Path(tempfile.mkdtemp(prefix="xidl-version-diff-"))
    context.add_cleanup(shutil.rmtree, temp_dir)
    reformatted_path = temp_dir / "version_diff.idl"
    reformatted_path.write_text(reformatted)
    original_result = run_diff(context, context.version_diff_source, "v1", "v2")
    reformatted_result = run_diff(context, reformatted_path, "v1", "v2")
    assert original_result.returncode == 1, original_result.stderr
    assert reformatted_result.returncode == 1, reformatted_result.stderr
    context.original_diff = original_result.stdout
    context.reformatted_diff = reformatted_result.stdout


@then("both version diffs are byte identical")
def same_canonical_diff(context):
    assert context.original_diff == context.reformatted_diff


@then("the version diff exits with 2 and lists the declared versions")
def unknown_version(context):
    result = context.version_diff_result
    assert result.returncode == 2
    assert result.stdout == ""
    assert "version missing is not declared" in result.stderr
    assert "v1 (2026-04), v2 (2026-07)" in result.stderr


@when("I compare invalid version diff contracts")
def invalid_contracts(context):
    temp_dir = Path(tempfile.mkdtemp(prefix="xidl-version-diff-invalid-"))
    context.add_cleanup(shutil.rmtree, temp_dir)
    source = context.version_diff_source.read_text()
    inputs = [
        ("unversioned.idl", "struct Item { long id; };"),
        ("syntax.idl", "struct Item {"),
        ("import.idl", 'import "missing.idl";\n' + source),
    ]
    context.invalid_diff_results = []
    for name, content in inputs:
        path = temp_dir / name
        path.write_text(content)
        context.invalid_diff_results.append((name, run_diff(context, path, "v1", "v2")))
    context.invalid_diff_results.append(
        ("absent.idl", run_diff(context, temp_dir / "absent.idl", "v1", "v2"))
    )


@then("every invalid contract exits with 2 and no diff")
def invalid_results(context):
    for name, result in context.invalid_diff_results:
        assert result.returncode == 2, result.stderr
        assert result.stdout == "", result.stdout
        assert result.stderr
        assert name in result.stderr, result.stderr
        assert "panicked" not in result.stderr


@when("I compare versions with a closed output pipe")
def compare_with_closed_pipe(context):
    with subprocess.Popen(
        diff_command(context.version_diff_source, "v1", "v2"),
        cwd=Path.cwd(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, "XIDLC_VERSION": "0.0.1", "XIDLC_GIT_HASH": "snapshot"},
    ) as process:
        process.stdout.close()
        process.stdout = None
        _, context.closed_pipe_error = process.communicate(timeout=180)
        context.closed_pipe_code = process.returncode


@then("the output failure exits with 2 without a panic")
def closed_pipe_failure(context):
    assert context.closed_pipe_code == 2, context.closed_pipe_error
    assert "cannot write output" in context.closed_pipe_error
    assert "version_diff.idl" in context.closed_pipe_error
    assert "panicked" not in context.closed_pipe_error
