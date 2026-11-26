"""BDD steps for xidlc version lock and version check."""

from pathlib import Path
import shutil
import subprocess
import tempfile

from behave import given, when, then


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
    result = subprocess.run(
        ["cargo", "build", "-p", "xidlc", "--features", "cli"],
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert result.returncode == 0, result.stderr
    context.version_lock_cli = Path.cwd() / "target/debug/xidlc"
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


@when("I append a second version with an added type to the contract")
def append_second_version(context):
    context.version_lock_idl.write_text(EXTENDED_IDL)


@when("I rename a field in the locked version without lifecycle annotations")
def rename_locked_field(context):
    source = context.version_lock_idl.read_text()
    context.version_lock_idl.write_text(source.replace("string name;", "string title;"))


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
    result = run_xidlc(context, "version", "lock", context.version_lock_idl)
    assert result.returncode == 1, (result.returncode, result.stdout, result.stderr)
    assert "--force" in result.stderr, result.stderr


@then("xidlc version lock with force updates the lock file")
def version_lock_with_force_succeeds(context):
    result = run_xidlc(context, "version", "lock", "--force", context.version_lock_idl)
    assert result.returncode == 0, (result.stdout, result.stderr)
