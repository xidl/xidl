"""Strict CLI regression checks for the user-supplied messaging corpus."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from behave import given, when, then


VERSIONS = ["v2026_03", "v2026_05", "v2026_07", "v2026_09", "v2026_11"]


def run_cli(context, *arguments):
    return subprocess.run(
        [str(context.messaging_cli), *map(str, arguments)],
        capture_output=True,
        text=True,
        timeout=120,
        cwd=Path.cwd(),
    )


def generate(context, source, output, target="openapi", version=None):
    arguments = ["gen", "--out-dir", output]
    if version is not None:
        arguments.extend(["--api-version", version])
    result = run_cli(context, *arguments, target, source)
    assert result.returncode == 0, (source, result.stdout, result.stderr)
    files = {p.name: p.read_bytes() for p in output.rglob("*") if p.is_file()}
    assert files, f"generation succeeded without artifacts: {source}"
    return files


@given("the five-version Relay messaging corpus")
def messaging_corpus(context):
    context.messaging_corpus = Path.cwd() / "bdd/features/data/relay_messaging"
    context.messaging_source = context.messaging_corpus / "relay_messaging.idl"
    context.messaging_work = Path(tempfile.mkdtemp(prefix="xidl-messaging-"))
    context.add_cleanup(shutil.rmtree, context.messaging_work)
    result = subprocess.run(
        ["cargo", "build", "-p", "xidlc", "--features", "cli"],
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert result.returncode == 0, result.stderr
    context.messaging_cli = Path.cwd() / "target/debug/xidlc"


@when("I generate and select every messaging release through the CLI")
def generate_history(context):
    context.messaging_history = generate(
        context, context.messaging_source, context.messaging_work / "all"
    )
    repeated = generate(
        context, context.messaging_source, context.messaging_work / "repeat"
    )
    assert context.messaging_history == repeated
    assert set(repeated) == {
        f"openapi_relay_messaging.{version}.json" for version in VERSIONS
    }


@then("each selected document matches its complete history byte for byte")
def selected_history(context):
    for version in VERSIONS:
        selected = generate(
            context,
            context.messaging_source,
            context.messaging_work / version,
            version=version,
        )
        name = f"openapi_relay_messaging.{version}.json"
        assert selected == {name: context.messaging_history[name]}

    original = generate(
        context, context.messaging_source, context.messaging_work / "map", "version-map"
    )
    trivia = context.messaging_work / "trivia" / context.messaging_source.name
    trivia.parent.mkdir()
    trivia.write_text(
        "\n/* moved comment */\n".join(
            context.messaging_source.read_text().splitlines()
        )
    )
    changed = generate(
        context,
        trivia,
        context.messaging_work / "trivia-map",
        "version-map",
        VERSIONS[-1],
    )
    assert original == changed
    encoded = original["relay_messaging.version-map.json"]
    document = json.loads(encoded)
    assert encoded == (json.dumps(document, indent=2) + "\n").encode()
    assert len(document["steps"]) == 4
    replacement = document["steps"][0]["operations"]["Relay::Messages.legacy_send"]
    assert replacement["from"]["declaration_id"] != replacement["to"]["declaration_id"]
    assert "Relay::Participant" not in document["steps"][1]["types"]


@then("canonical diffs preserve the messaging lifecycle boundaries")
def history_diff(context):
    for version in VERSIONS:
        same = run_cli(
            context, "version", "diff", version, version, context.messaging_source
        )
        assert same.returncode == 0, same.stderr
        assert same.stdout == ""
    changed = run_cli(
        context, "version", "diff", "2026-07", "2026-09", context.messaging_source
    )
    assert changed.returncode == 1, changed.stderr
    assert "--- relay_messaging.idl @ v2026_07 (2026-07)" in changed.stdout
    assert "+++ relay_messaging.idl @ v2026_09 (2026-09)" in changed.stdout
    assert "Idempotency-Key" in changed.stdout
    assert (
        "created_at_epoch" in changed.stdout and "created_at_rfc3339" in changed.stdout
    )
    assert "@server_stream" in changed.stdout


@then("a historical edit changes only the releases containing that declaration")
def historical_edit(context):
    lock = context.messaging_corpus / "expected/versions.lock.json"
    historical_lock = lock.read_bytes()
    altered = context.messaging_work / context.messaging_source.name
    source = context.messaging_source.read_text()
    assert source.count("string mime_type;") == 1
    altered.write_text(source.replace("string mime_type;", "string content_type;"))
    # Generation must succeed before changed digests can count as detected drift.
    changed = generate(context, altered, context.messaging_work / "altered")
    assert set(changed) == set(context.messaging_history)
    for version in VERSIONS:
        name = f"openapi_relay_messaging.{version}.json"
        before = hashlib.sha256(context.messaging_history[name]).digest()
        after = hashlib.sha256(changed[name]).digest()
        assert (before == after) == (version == "v2026_03"), version
    assert lock.read_bytes() == historical_lock


@then("every negative messaging input has its documented CLI outcome")
def negative_history(context):
    expectations = json.loads(
        (context.messaging_corpus / "expected/negative.json").read_text()
    )
    for name, expected in expectations.items():
        source = context.messaging_corpus / "negative" / f"{name}.idl"
        output = context.messaging_work / name
        if expected["expect"] == "lint-error":
            lint = run_cli(context, "lint", source)
            assert lint.returncode == 1, (name, lint.stderr)
            assert expected["contains"] in lint.stdout + lint.stderr, (
                name,
                lint.stderr,
            )
            generate(context, source, output)
        elif expected["expect"] == "gen-error":
            result = run_cli(context, "gen", "-o", output, "openapi", source)
            assert result.returncode != 0, name
            combined = result.stdout + result.stderr
            assert expected["contains"] in combined, (name, combined)
            assert "panicked" not in combined
            assert not output.exists() or not any(output.rglob("*")), name
        elif expected["expect"] == "header-name":
            files = generate(context, source, output)
            for encoded in files.values():
                document = json.loads(encoded)
                parameters = document["paths"]["/h"]["get"]["parameters"]
                assert parameters == [
                    {
                        "in": "header",
                        "name": expected["value"],
                        "required": False,
                        "schema": {"type": "string"},
                    }
                ]
        elif expected["expect"] == "errors-by-version":
            files = generate(context, source, output)
            assert len(files) == len(expected["value"])
            for version, errors in expected["value"].items():
                document = json.loads(files[f"openapi_{name}.{version}.json"])
                operation = document["paths"]["/g"]["get"]
                statuses = set(operation["responses"])
                assert statuses == {"200", "500", *errors}, (version, statuses)
        else:
            raise AssertionError(f"unhandled expectation: {name}: {expected}")
    positive = generate(
        context,
        context.messaging_corpus / "negative/P01_unversioned_union_inherit.idl",
        context.messaging_work / "P01",
    )
    assert len(positive) == 1
    document = json.loads(next(iter(positive.values())))
    assert set(document["paths"]) == {"/r", "/p"}
    assert len(document["components"]["schemas"]["Relay.Reaction"]["oneOf"]) == 2


@then("unknown release selections fail without producing artifacts")
def unknown_release(context):
    output = context.messaging_work / "unknown"
    result = run_cli(
        context,
        "gen",
        "--api-version",
        "v2027_01",
        "-o",
        output,
        "openapi",
        context.messaging_source,
    )
    assert result.returncode != 0
    assert "v2027_01" in result.stderr
    assert not output.exists() or not any(output.rglob("*"))
    diff = run_cli(
        context, "version", "diff", "v2027_01", VERSIONS[0], context.messaging_source
    )
    assert diff.returncode == 2
    assert diff.stdout == ""
    for version in VERSIONS:
        assert version in diff.stderr
