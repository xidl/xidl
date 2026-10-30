import shutil
import subprocess
import os
from pathlib import Path

from behave import then, when


def run(command, *, cwd):
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    assert result.returncode == 0, result.stdout + result.stderr
    return result


@when("I generate TypeScript with the independent adapter target")
def generate_independent_target(context):
    context.execute_steps("When I generate ts code for the IDL")
    help_result = run(
        [
            str(
                Path.cwd()
                / os.environ.get("CARGO_TARGET_DIR", "target")
                / "debug/xidlc"
            ),
            "gen",
            "--help",
        ],
        cwd=Path.cwd(),
    )
    assert "typescript-rest-adapters" in help_result.stdout
    generate_adapter_artifact(context, "typescript-rest-adapters")


@when("I generate Rust with the independent adapter target")
def generate_rust_target(context):
    context.execute_steps("When I generate rust code for the IDL")
    generate_adapter_artifact(context, "rust-axum-adapters")


def generate_adapter_artifact(context, target):
    output = Path(context.lang_dir)
    context.ordinary_adapter_baseline = {
        path.name: path.read_bytes() for path in output.iterdir() if path.is_file()
    }
    command = [
        str(Path.cwd() / os.environ.get("CARGO_TARGET_DIR", "target") / "debug/xidlc"),
        "gen",
        "--out-dir",
        str(output),
        target,
        context.idl_file,
    ]
    run(command, cwd=Path.cwd())
    first = {
        path.name: path.read_bytes() for path in output.iterdir() if path.is_file()
    }
    run(command, cwd=Path.cwd())
    second = {
        path.name: path.read_bytes() for path in output.iterdir() if path.is_file()
    }
    assert first == second, "independent adapter output must be byte-stable"
    assert len(first) == len(context.ordinary_adapter_baseline) + 1
    for name, content in context.ordinary_adapter_baseline.items():
        assert first[name] == content, (
            f"adapter target overwrote ordinary artifact {name}"
        )


def prepare_application(context, script, boilerplate_name="versioned_adapters"):
    root = Path.cwd()
    output = Path(context.lang_dir)
    boilerplate = root / "bdd/boilerplate" / boilerplate_name / "ts"
    package = (boilerplate / "package.json").read_text()
    for name in ["CLIENT", "CODEC", "SERVER"]:
        package = package.replace(
            "{{TS_XIDL_TYPESCRIPT_" + name + "_PATH}}",
            str(root / "typescript" / ("xidl-typescript-" + name.lower())),
        )
    (output / "package.json").write_text(package)
    shutil.copy(boilerplate / "tsconfig.json", output)
    shutil.copy(boilerplate / script, output)
    companions = {
        "business.ts": [
            "business_dispatch.ts",
            "business_hook_contracts.ts",
            "business_http.ts",
            "business_stored.ts",
            "business_update.ts",
        ],
        "chain.ts": ["chain_http.ts", "chain_app.ts"],
        "transforms.ts": [
            "transforms_http.ts",
            "transforms_app.ts",
            "transforms_hooks.ts",
        ],
    }
    for name in companions.get(script, []):
        shutil.copy(boilerplate / name, output)
    run(["pnpm", "install", "--ignore-scripts", "--no-frozen-lockfile"], cwd=output)
    run(["pnpm", "exec", "tsc", "--noEmit"], cwd=output)
    return output


@then("one TypeScript Head implementation passes the unchanged adapter Hurl contracts")
def existing_hurl_contracts(context):
    output = prepare_application(context, "legacy.ts")
    result = run(
        ["pnpm", "exec", "tsx", "legacy.ts", str(Path(context.idl_file).parent)],
        cwd=output,
    )
    assert "PASS: unchanged" in result.stdout


def expect_tsc_failure(output, name, content, needle, reason):
    """Compile a deliberately incomplete module and require a typed error."""
    path = output / name
    path.write_text(content)
    try:
        result = subprocess.run(
            ["pnpm", "exec", "tsc", "--noEmit"],
            cwd=output,
            capture_output=True,
            text=True,
        )
        assert result.returncode != 0, reason
        assert needle in result.stdout + result.stderr, result.stdout + result.stderr
    finally:
        path.unlink()


@then("the three-version TypeScript application enforces wire and business boundaries")
def real_business_journey(context):
    output = prepare_application(context, "business.ts")
    result = run(["pnpm", "exec", "tsx", "business.ts"], cwd=output)
    assert "PASS: three-version" in result.stdout
    adapters = "./versioned_adapters_business.adapters"
    expect_tsc_failure(
        output,
        "missing-hook.ts",
        f'import type {{ VersionAdapters }} from "{adapters}";\n'
        "export const hooks: VersionAdapters = {};\n",
        "parcelUpcastFromV1",
        "missing business decisions must fail tsc",
    )
    expect_tsc_failure(
        output,
        "missing-reader.ts",
        f'import type {{ StoredValues }} from "{adapters}";\n'
        "export const stored: StoredValues = {};\n",
        "parcelHandlingNote",
        "every retired member needs a stored value reader",
    )
    expect_tsc_failure(
        output,
        "silent-drop.ts",
        f'import type {{ VersionAdapters }} from "{adapters}";\n'
        "export const partial: Pick<VersionAdapters, 'parcelDowncastToV2'> = {\n"
        "  parcelDowncastToV2: () => ({ ok: true, value: {} }),\n"
        "};\n",
        "handling_note",
        "a retired optional member cannot be dropped silently",
    )
    expect_tsc_failure(
        output,
        "implied-default.ts",
        f'import type {{ VersionAdapters }} from "{adapters}";\n'
        "export const partial: Pick<VersionAdapters, 'parcelUpcastFromV2'> = {\n"
        "  parcelUpcastFromV2: () =>\n"
        "    ({ ok: true, value: { audit: 'checked', handling_note: 'xidl.ignored' as const } }),\n"
        "};\n",
        "signature_required",
        "a defaulted addition needs the value older versions implied",
    )
    expect_tsc_failure(
        output,
        "missing-persist.ts",
        f'import type {{ VersionAdapters }} from "{adapters}";\n'
        "import type * as types from './versioned_adapters_business';\n"
        "export const patch: Pick<VersionAdapters, 'parcelPatchUpcastFromV2'> = {\n"
        "  parcelPatchUpcastFromV2: (source: types.v2.ParcelPatch) =>\n"
        "    ({ ok: true, value: { handling_note: 'xidl.ignored' as const } }),\n"
        "};\n",
        "stored",
        "a paired patch decision must return the members its handler persists",
    )
    expect_tsc_failure(
        output,
        "handler-without-stored.ts",
        f'import * as a from "{adapters}";\n'
        "import type * as types from './versioned_adapters_business';\n"
        "export async function broken(\n"
        "  current: types.v3.Parcel,\n"
        "  adapters: a.VersionAdapters,\n"
        ") {\n"
        "  return a.dispatchDepotUpdate(\n"
        "    'v2',\n"
        "    { id: 'p1', patch: {} },\n"
        "    current,\n"
        "    async () => ({ ok: true as const, value: current }),\n"
        "    adapters,\n"
        "  );\n"
        "}\n",
        "stored",
        "an update handler must return the stored values its outcome still reads",
    )


@then("the single-version TypeScript application validates before dispatch")
def single_version_journey(context):
    output = prepare_application(context, "single.ts")
    result = run(["pnpm", "exec", "tsx", "single.ts"], cwd=output)
    assert "PASS: single-version" in result.stdout


def prepare_rust_application(context, script, boilerplate_name="versioned_adapters"):
    root = Path.cwd()
    output = Path(context.lang_dir)
    boilerplate = root / "bdd/boilerplate" / boilerplate_name / "rust"
    (output / "src").mkdir(exist_ok=True)
    package = (
        (boilerplate / "Cargo.toml")
        .read_text()
        .replace("{{RUST_XIDL_RUST_AXUM_PATH}}", str(root / "crates/xidl-rust-axum"))
    )
    (output / "Cargo.toml").write_text(package)
    shutil.copy(boilerplate / script, output / "src/main.rs")
    companions = {
        "versioned_adapters": {"main.rs": ["hooks.rs", "journey.rs", "wire.rs"]},
        "versioned_chain": {
            "main.rs": ["hooks.rs", "chain.rs", "http.rs", "http_app.rs"]
        },
        "versioned_events": {"main.rs": ["events.rs"]},
        "versioned_transforms": {
            "main.rs": [
                "hooks.rs",
                "transforms.rs",
                "http.rs",
                "http_app.rs",
                "http_roster.rs",
                "routes.rs",
            ]
        },
    }
    for name in companions.get(boilerplate_name, {}).get(script, []):
        shutil.copy(boilerplate / name, output / "src" / name)
    return output


@then("one Rust Head implementation passes the unchanged adapter Hurl contracts")
def rust_frozen_http_contracts(context):
    output = prepare_rust_application(context, "legacy.rs")
    result = run(
        ["cargo", "run", "--quiet", "--", str(Path(context.idl_file).parent.resolve())],
        cwd=output,
    )
    assert "PASS: unchanged Hurl" in result.stdout


def expect_cargo_check_failure(output, path, content, needle, reason):
    """Write a deliberately incomplete source file and require a typed compile error."""
    original = path.read_text()
    path.write_text(content)
    try:
        result = subprocess.run(
            ["cargo", "check", "--quiet"], cwd=output, capture_output=True, text=True
        )
        assert result.returncode != 0, reason
        assert needle in result.stderr, result.stderr
    finally:
        path.write_text(original)


@then("the three-version Rust application enforces wire and business boundaries")
def rust_business_journey(context):
    output = prepare_rust_application(context, "main.rs")
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: Rust three-version" in result.stdout
    main = output / "src/main.rs"
    hooks = output / "src/hooks.rs"
    expect_cargo_check_failure(
        output,
        main,
        main.read_text()
        + "\nstruct Missing;\nimpl gen::version_adapters::VersionAdapters for Missing {}\n",
        "parcel_upcast_from_v_1",
        "missing business hooks must fail cargo check",
    )
    expect_cargo_check_failure(
        output,
        hooks,
        hooks.read_text().replace("legacy: IGNORED,", ""),
        "missing field `legacy`",
        "discarded request members require IGNORED",
    )
    expect_cargo_check_failure(
        output,
        main,
        main.read_text()
        + "\nstruct Forgetful;\nimpl gen::version_adapters::StoredValues for Forgetful {}\n",
        "parcel_handling_note",
        "every retired member needs a stored value reader",
    )
    expect_cargo_check_failure(
        output,
        hooks,
        hooks.read_text().replace(
            "handling_note: stored.parcel_handling_note(source)", ""
        ),
        "missing field `handling_note`",
        "a retired optional member cannot be dropped silently",
    )
    expect_cargo_check_failure(
        output,
        hooks,
        hooks.read_text().replace(", signature_required: false", ""),
        "missing field `signature_required`",
        "a defaulted addition needs the value older versions implied",
    )
    expect_cargo_check_failure(
        output,
        hooks,
        hooks.read_text().replace(
            "stored: DepotUpdateStored { handling_note: source.handling_note.clone() },",
            "",
        ),
        "missing field `stored`",
        "a paired patch decision must return the members its handler persists",
    )
    expect_cargo_check_failure(
        output,
        hooks,
        hooks.read_text().replace(
            "fn parcel_patch_upcast_from_v_2(&self, source: &v2::ParcelPatch, _current: &v3::Parcel)",
            "fn parcel_patch_upcast_from_v_2(&self, source: &v2::ParcelPatch, _current: &v2::Parcel)",
        ),
        "v3::Parcel",
        "the paired patch hook receives the Head return type as current",
    )
    journey = output / "src/journey.rs"
    expect_cargo_check_failure(
        output,
        journey,
        journey.read_text()
        + "\nasync fn forgetful(current: crate::gen::v3::Parcel) {\n"
        "    let _ = dispatch_depot_update(Version::V2, serde_json::json!({}), current,\n"
        "        |_, _| async { unreachable!() }, &crate::hooks::Hooks::default()).await;\n"
        "}\n",
        "takes 3 arguments",
        "an update handler must accept the persisted-member bag",
    )


@then("the single-version Rust application validates before dispatch")
def rust_single_version_journey(context):
    output = prepare_rust_application(context, "single.rs")
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: Rust single-version" in result.stdout


@then("the versioned booking chain keeps storage facts and pure conversions")
def versioned_chain_journey(context):
    output = prepare_application(
        context, "chain.ts", boilerplate_name="versioned_chain"
    )
    result = run(["pnpm", "exec", "tsx", "chain.ts"], cwd=output)
    assert "PASS: booking chain" in result.stdout


@then("the Rust versioned booking chain keeps the same facts")
def rust_versioned_chain_journey(context):
    output = prepare_rust_application(
        context, "main.rs", boilerplate_name="versioned_chain"
    )
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: booking chain" in result.stdout


@then("the versioned event split keeps old names, distinct ids and one stated half")
def versioned_events_journey(context):
    output = prepare_application(
        context, "events.ts", boilerplate_name="versioned_events"
    )
    result = run(["pnpm", "exec", "tsx", "events.ts"], cwd=output)
    assert "PASS: event split" in result.stdout


@then("the Rust versioned event split keeps the same promises")
def rust_versioned_events_journey(context):
    output = prepare_rust_application(
        context, "main.rs", boilerplate_name="versioned_events"
    )
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: event split" in result.stdout


@then("the versioned transform shapes upcast, downcast and round-trip")
def versioned_transforms_journey(context):
    output = prepare_application(
        context, "transforms.ts", boilerplate_name="versioned_transforms"
    )
    result = run(["pnpm", "exec", "tsx", "transforms.ts"], cwd=output)
    assert "PASS: transform shapes" in result.stdout


@then("the Rust versioned transform shapes keep the same promises")
def rust_versioned_transforms_journey(context):
    output = prepare_rust_application(
        context, "main.rs", boilerplate_name="versioned_transforms"
    )
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: transform shapes" in result.stdout
