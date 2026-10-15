"""Steps for the version freeze release gate.

The steps run the real `xidlc version` subcommands against a scratch copy of
the contract, so every scenario exercises the frozen copies, the drift report
and the exit codes a release gate actually sees.
"""

import json
import os
import shutil
import subprocess
import tempfile

from behave import given, then, when


def _run_xidlc(context, extra_args):
    cmd = ["cargo", "run", "-p", "xidlc", "--features", "cli,fmt", "--"]
    cmd.extend(extra_args)
    return subprocess.run(
        cmd + [context.contract_path],
        capture_output=True,
        text=True,
        cwd=context.freeze_dir,
    )


def _run(context, extra_args, expect_success):
    result = _run_xidlc(context, extra_args)
    context.last_result = result
    if expect_success:
        assert result.returncode == 0, (
            f"xidlc {extra_args} failed:\n{result.stdout}\n{result.stderr}"
        )
    else:
        assert result.returncode != 0, (
            f"expected xidlc {extra_args} to fail:\n{result.stdout}"
        )
        context.failure = result.stdout + result.stderr
    return result


def _edit_contract(context, old, new):
    with open(context.contract_path, "r") as handle:
        source = handle.read()
    assert old in source, f"fixture lost its edit anchor: {old}"
    with open(context.contract_path, "w") as handle:
        handle.write(source.replace(old, new))


@given('a versioned contract file "{idl_file}" in a scratch workspace')
def step_impl(context, idl_file):
    base_temp = os.path.join(os.getcwd(), "bdd", ".temp")
    os.makedirs(base_temp, exist_ok=True)
    context.freeze_dir = tempfile.mkdtemp(dir=base_temp)
    context.contract_path = os.path.join(context.freeze_dir, "store.idl")
    shutil.copy(idl_file, context.contract_path)
    context.copies_dir = os.path.join(context.freeze_dir, "store.versions")


def _freeze(context, expect_success=True):
    _run(context, ["version", "verify", "--update"], expect_success)


@when("I freeze the contract")
def step_impl(context):
    _freeze(context)


@when("I freeze the contract and expect failure")
def step_impl(context):
    _freeze(context, expect_success=False)


@when("I verify the frozen contract")
def step_impl(context):
    _run(context, ["version", "verify"], True)


@when("I verify the frozen contract and expect failure")
def step_impl(context):
    _run(context, ["version", "verify"], False)


@then("the release gate passes")
def step_impl(context):
    assert context.last_result.returncode == 0, context.last_result.stderr


@then("the frozen copies exist for every version value")
def step_impl(context):
    with open(os.path.join(context.copies_dir, "index.json"), "r") as handle:
        index = json.load(handle)
    assert index["schema"] == "xidl.versions-index/v1", index
    assert index["released"], index
    copies = sorted(
        name[:-5]
        for name in os.listdir(context.copies_dir)
        if name.endswith(".json") and name != "index.json"
    )
    assert sorted(index["released"]) == copies, index
    for value in index["released"]:
        assert os.path.exists(os.path.join(context.copies_dir, f"{value}.json")), (
            f"missing frozen copy for {value}"
        )
        with open(os.path.join(context.copies_dir, f"{value}.json"), "r") as handle:
            document = json.load(handle)
        assert document["info"]["version"] == value, document.get("info")


@then("the frozen copies are byte-for-byte unchanged")
def step_impl(context):
    for value in ("2025-01", "2025-06"):
        path = os.path.join(context.copies_dir, f"{value}.json")
        with open(path, "rb") as handle:
            context.frozen_bytes = context.frozen_bytes or {}
        assert path in context.frozen_bytes, (
            "the first freeze should have recorded bytes"
        )
        with open(path, "rb") as handle:
            assert handle.read() == context.frozen_bytes[path], f"{value} drifted"


@when("I freeze the contract again")
def step_impl(context):
    context.frozen_bytes = getattr(context, "frozen_bytes", {}) or {}
    for value in ("2025-01", "2025-06"):
        path = os.path.join(context.copies_dir, f"{value}.json")
        with open(path, "rb") as handle:
            context.frozen_bytes[path] = handle.read()
    _freeze(context)


@then("the failure tells how to create the frozen copies")
def step_impl(context):
    assert "no frozen versions under" in context.failure, context.failure
    assert "xidlc version verify --update" in context.failure, context.failure


@when("I add a comment to the contract")
def step_impl(context):
    with open(context.contract_path, "a") as handle:
        handle.write("\n// a comment that changes nothing on the wire\n")


@when("I rename the released operation")
def step_impl(context):
    _edit_contract(
        context, "Item get(@path string id);", "Item fetch(@path string id);"
    )


@then("the drift report names the released version and the changed path")
def step_impl(context):
    assert "released version 2025-01 changed:" in context.failure, context.failure
    assert "/paths//items/{id}/get/operationId" in context.failure, context.failure


@when('I change the released version value "{old}" to "{new}"')
def step_impl(context, old, new):
    _edit_contract(context, f'@rename("{old}")', f'@rename("{new}")')


@then("the drift report says the released version disappeared")
def step_impl(context):
    assert "released version 2025-06 disappeared" in context.failure, context.failure
    assert "is not frozen yet" in context.failure, context.failure


@when('I append the version "{value}"')
def step_impl(context, value):
    _edit_contract(
        context,
        '    @rename("2025-06") v2,',
        f'    @rename("2025-06") v2,\n    @rename("{value}") v3,',
    )


@then("the failure offers the update door")
def step_impl(context):
    assert "new version v3 (2026-01)" in context.failure, context.failure
    assert "xidlc version verify --update" in context.failure, context.failure


@when('I insert the version "{value}" before the released ones')
def step_impl(context, value):
    _edit_contract(
        context,
        '    @rename("2025-01") v1,',
        f'    @rename("{value}") v0,\n    @rename("2025-01") v1,',
    )


@then("the failure says released history is append-only")
def step_impl(context):
    assert "is inserted before released version" in context.failure, context.failure
    assert "append-only" in context.failure, context.failure


@when('I run "version diff" for "{older}" to "{newer}"')
def step_impl(context, older, newer):
    _run(
        context,
        ["version", "diff", "--from", older, "--to", newer],
        True,
    )


@then("the diff prints the structural difference")
def step_impl(context):
    out = context.last_result.stdout
    assert "2025-01 → 2025-06" in out, out
    assert "+ /components/schemas/v2.Item:" in out, out
    assert "~ /info/version: 2025-01 → 2025-06" in out, out
