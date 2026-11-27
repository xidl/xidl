"""BDD steps for xidlc version lock and version check."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from behave import given, when, then

_XIDLC_BIN = None


def ensure_xidlc_binary() -> Path:
    global _XIDLC_BIN
    if _XIDLC_BIN is not None and _XIDLC_BIN.exists():
        return _XIDLC_BIN
    result = subprocess.run(
        ["cargo", "build", "-p", "xidlc", "--features", "cli"],
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert result.returncode == 0, result.stderr
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", Path.cwd() / "target"))
    binary = target_dir / "debug" / "xidlc"
    assert binary.exists(), f"expected built xidlc binary at {binary}"
    _XIDLC_BIN = binary
    return binary


INITIAL_IDL = """\
@version
enum V {
    @rename("2026-12-01") v1
};

struct Workspace {
    string id;
    string name;
};

interface Api {
    @get(path = "/workspaces")
    Workspace get();
};
"""

EXTENDED_IDL = """\
@version
enum V {
    @rename("2026-12-01") v1,
    @rename("2027-06-01") v2
};

struct Workspace {
    string id;
    string name;
};

@added(V::v2)
struct AuditEntry {
    string actor;
};

interface Api {
    @get(path = "/workspaces")
    Workspace get();
};
"""


def run_xidlc(context, *arguments):
    return subprocess.run(
        [str(context.version_lock_cli), *map(str, arguments)],
        capture_output=True,
        text=True,
        timeout=120,
        cwd=Path.cwd(),
    )


@given("a temporary workspace with a single-version IDL contract")
def setup_version_lock_workspace(context):
    context.version_lock_cli = ensure_xidlc_binary()
    context.version_lock_dir = Path(tempfile.mkdtemp(prefix="xidl-version-lock-"))
    context.add_cleanup(shutil.rmtree, context.version_lock_dir)
    context.version_lock_idl = context.version_lock_dir / "workspace.idl"
    context.version_lock_idl.write_text(INITIAL_IDL)


@when("I lock the contract versions with xidlc version lock")
def lock_contract_versions(context):
    result = run_xidlc(context, "version", "lock", context.version_lock_idl)
    assert result.returncode == 0, (result.stdout, result.stderr)
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    assert lock_file.exists(), "expected default lock file to be created"
    doc = json.loads(lock_file.read_text())
    assert doc["schema_version"] == 1, doc
    assert "2026-12-01" in doc["versions"], doc


@when("I append a second version with an added type to the contract")
def append_second_version(context):
    context.version_lock_idl.write_text(EXTENDED_IDL)


@when("I rename a field in the locked version without lifecycle annotations")
def rename_locked_field(context):
    source = context.version_lock_idl.read_text()
    assert "string name;" in source, "expected fixture to contain 'string name;'"
    context.version_lock_idl.write_text(
        source.replace("string name;", "string title;", 1)
    )


@then("xidlc version check succeeds with exit code 0")
def version_check_succeeds(context):
    result = run_xidlc(context, "version", "check", context.version_lock_idl)
    assert result.returncode == 0, (result.stdout, result.stderr)


@then("xidlc version check fails with exit code 1 reporting the drifted version")
def version_check_fails(context):
    result = run_xidlc(context, "version", "check", context.version_lock_idl)
    assert result.returncode == 1, (result.returncode, result.stdout, result.stderr)
    assert "2026-12-01" in result.stderr, result.stderr


@then("xidlc version lock without force fails with exit code 1")
def version_lock_without_force_fails(context):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    before = lock_file.read_text()
    result = run_xidlc(context, "version", "lock", context.version_lock_idl)
    assert result.returncode == 1, (result.returncode, result.stdout, result.stderr)
    assert "--force" in result.stderr, result.stderr
    assert lock_file.read_text() == before, "unforced lock modified existing lock file"
    context.pre_force_lock_content = before


@then("xidlc version lock with force updates the lock file")
def version_lock_with_force_succeeds(context):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    result = run_xidlc(context, "version", "lock", "--force", context.version_lock_idl)
    assert result.returncode == 0, (result.stdout, result.stderr)
    after = lock_file.read_text()
    assert after != context.pre_force_lock_content, (
        "forced lock did not update fingerprint"
    )


@when('I lock only version "{version}"')
def lock_one_version(context, version):
    result = run_xidlc(
        context,
        "version",
        "lock",
        "--versions",
        json.dumps([version]),
        context.version_lock_idl,
    )
    assert result.returncode == 0, (result.stdout, result.stderr)


@then('the lock contains only "{version}"')
def lock_has_only_version(context, version):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    assert list(json.loads(lock_file.read_text())["versions"]) == [version]


@when("I rename a field added in the second version")
def rename_second_version_field(context):
    source = context.version_lock_idl.read_text()
    assert "string actor;" in source
    context.version_lock_idl.write_text(
        source.replace("string actor;", "string author;")
    )


@then('checking only version "{version}" succeeds')
def check_one_version(context, version):
    result = run_xidlc(
        context,
        "version",
        "check",
        "--versions",
        json.dumps([version]),
        context.version_lock_idl,
    )
    assert result.returncode == 0, (result.stdout, result.stderr)


@then('checking only version "{version}" reports it is not locked')
def check_unlocked_version(context, version):
    result = run_xidlc(
        context,
        "version",
        "check",
        "--versions",
        json.dumps([version]),
        context.version_lock_idl,
    )
    assert result.returncode == 1, (result.stdout, result.stderr)
    assert (
        version in result.stderr and "is not present in the lock file" in result.stderr
    )


@when("I lock the explicit empty version set")
def lock_empty_version_set(context):
    result = run_xidlc(
        context, "version", "lock", "--versions", "[]", context.version_lock_idl
    )
    assert result.returncode == 0, (result.stdout, result.stderr)


@then("the lock contains no versions and the empty version check succeeds")
def check_empty_version_set(context):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    assert json.loads(lock_file.read_text())["versions"] == {}
    result = run_xidlc(
        context, "version", "check", "--versions", "[]", context.version_lock_idl
    )
    assert result.returncode == 0, (result.stdout, result.stderr)


@when("I lock the generated OpenAPI contract")
def lock_openapi(context):
    result = run_xidlc(
        context, "version", "lock", "--artifact", "openapi", context.version_lock_idl
    )
    assert result.returncode == 0, (result.stdout, result.stderr)
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    version = json.loads(lock_file.read_text())["versions"]["2026-12-01"]
    assert version["openapi_fingerprint"].startswith("sha256:")


@when("I document and deprecate a field in the contract")
def document_and_deprecate_field(context):
    source = context.version_lock_idl.read_text()
    assert "    string name;" in source
    context.version_lock_idl.write_text(
        source.replace(
            "    string name;", "    /// Display name.\n    @deprecated string name;"
        )
    )


@then("the OpenAPI fingerprint still matches")
def check_openapi(context):
    result = run_xidlc(
        context, "version", "check", "--artifact", "openapi", context.version_lock_idl
    )
    assert result.returncode == 0, (result.stdout, result.stderr)


@then("checking the OpenAPI fingerprint reports drift")
def check_openapi_drift(context):
    result = run_xidlc(
        context, "version", "check", "--artifact", "openapi", context.version_lock_idl
    )
    assert result.returncode == 1, (result.stdout, result.stderr)
    assert "OpenAPI artifact changed" in result.stderr


@when("I remove the algorithm version from the lock")
def remove_lock_algorithm_version(context):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    document = json.loads(lock_file.read_text())
    del document["algorithm_version"]
    lock_file.write_text(json.dumps(document))


@then("xidlc diagnoses the old lock before comparing fingerprints")
def old_lock_needs_regeneration(context):
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    before = lock_file.read_bytes()
    for command in ["check", "lock"]:
        result = run_xidlc(context, "version", command, context.version_lock_idl)
        assert result.returncode == 2, (result.stdout, result.stderr)
        assert result.stderr.count("version lock --force") == 1, result.stderr
        assert "version lock mismatch" not in result.stderr, result.stderr
        assert lock_file.read_bytes() == before


@when("I explicitly regenerate the lock")
def force_regenerate_old_lock(context):
    result = run_xidlc(context, "version", "lock", "--force", context.version_lock_idl)
    assert result.returncode == 0, (result.stdout, result.stderr)
    lock_file = context.version_lock_dir / "workspace.versions.lock.json"
    doc = json.loads(lock_file.read_text())
    assert doc["algorithm_version"] > 0
    assert doc["versions"]["2026-12-01"]["openapi_fingerprint"].startswith("sha256:")
