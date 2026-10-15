"""Steps for the version spine release gate: manifest, freeze, diff.

The spine steps run the real `xidlc version` subcommands against a scratch
copy of the contract, so every scenario exercises the lock file, the drift
report and the exit codes a release gate actually sees.
"""

import json
import os
import shlex
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
        cwd=context.spine_dir,
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
    context.spine_dir = tempfile.mkdtemp(dir=base_temp)
    context.contract_path = os.path.join(context.spine_dir, "store.idl")
    shutil.copy(idl_file, context.contract_path)
    context.lock_path = os.path.join(context.spine_dir, "store.versions.lock.json")


@when('I run "version {subcommand}" on the contract')
def step_impl(context, subcommand):
    _run(context, shlex.split(f"version {subcommand}"), expect_success=True)


@when('I run "version {subcommand}" on the contract and expect failure')
def step_impl(context, subcommand):
    _run(context, shlex.split(f"version {subcommand}"), expect_success=False)


@when("I freeze the contract")
def step_impl(context):
    _run(context, ["version", "verify", "--update"], expect_success=True)


@when("I freeze the contract again")
def step_impl(context):
    with open(context.lock_path, "rb") as handle:
        context.lock_bytes = handle.read()
    _run(context, ["version", "verify", "--update"], expect_success=True)


@when("I verify the frozen contract")
def step_impl(context):
    _run(context, ["version", "verify"], expect_success=True)


@when("I verify the frozen contract and expect failure")
def step_impl(context):
    _run(context, ["version", "verify"], expect_success=False)


@when("I edit the contract so the v2 surface drifts")
def step_impl(context):
    _edit_contract(context, "@optional string note;", "@optional long note;")


@when("I edit the contract so the selector header renames")
def step_impl(context):
    _edit_contract(
        context,
        '@version_selector(header = "Api-Version")',
        '@version_selector(header = "X-Spine-Version")',
    )


@when("I edit the contract so the v2 wire value changes")
def step_impl(context):
    _edit_contract(context, '@rename("2025-06") v2', '@rename("2025-07") v2')


@when("I tamper with the frozen digest in the lock file")
def step_impl(context):
    with open(context.lock_path, "r") as handle:
        lock = json.load(handle)
    digest = lock["contracts"]["store"]["versions"]["v1"]["digest"]
    lock["contracts"]["store"]["versions"]["v1"]["digest"] = digest[:-1] + (
        "1" if digest.endswith("0") else "0"
    )
    with open(context.lock_path, "w") as handle:
        json.dump(lock, handle, indent=2)


@then("the lock file exists beside the contract")
def step_impl(context):
    assert os.path.exists(context.lock_path), "the freeze did not write the lock"


@then("the lock file is byte-for-byte unchanged")
def step_impl(context):
    with open(context.lock_path, "rb") as handle:
        assert handle.read() == context.lock_bytes, "re-freezing churned the lock"


@then("the release gate passes")
def step_impl(context):
    assert context.last_result.returncode == 0


@then("the manifest pins the selector, the timeline and both surface digests")
def step_impl(context):
    spine = json.loads(context.last_result.stdout)["spine"]
    assert spine["selector"] == {"location": "header", "name": "Api-Version"}, spine[
        "selector"
    ]
    assert [entry["ident"] for entry in spine["timeline"]] == ["v1", "v2"]
    digests = [surface["digest"] for surface in spine["surfaces"]]
    assert len(digests) == 2, digests
    assert all(digest.startswith("md5:") for digest in digests), digests
    context.spine = spine


@then("the manifest records how the versions correspond")
def step_impl(context):
    cells = context.spine["correspondence"]
    assert cells, "the spine recorded no correspondence"
    assert all(cell["from"] == "v1" and cell["to"] == "v2" for cell in cells), cells
    assert any(cell["kind"] == "added" for cell in cells), cells


@then("the failure tells how to create the lock")
def step_impl(context):
    assert "no lock file" in context.failure, context.failure
    assert "xidlc version verify --update" in context.failure, context.failure


@then("the drift report names the released version and both digests")
def step_impl(context):
    assert "release freeze violated" in context.failure, context.failure
    assert "released version v2 changed: md5:" in context.failure, context.failure
    assert "→ md5:" in context.failure, context.failure


@then("the drift report names the tampered digest and the real one")
def step_impl(context):
    assert "release freeze violated" in context.failure, context.failure
    assert "released version v1 changed: md5:" in context.failure, context.failure


@then("the drift report names the selector change")
def step_impl(context):
    assert "version selector changed" in context.failure, context.failure


@then("the drift report names the wire value change")
def step_impl(context):
    assert (
        "released version v2 changed its wire value: 2025-06 → 2025-07"
        in context.failure
    ), context.failure


@then("the report shows the rename with its explicit-rename evidence")
def step_impl(context):
    out = context.last_result.stdout
    assert "renamed" in out, out
    assert "[explicit-rename]" in out, out


@then("the report flags the wire coincidence for review")
def step_impl(context):
    out = context.last_result.stdout
    assert "name-shifted" in out, out
    assert "review before adapters rely on it" in out, out


@then("the failure says the timeline runs forward only")
def step_impl(context):
    assert "runs forward in the timeline" in context.failure, context.failure
