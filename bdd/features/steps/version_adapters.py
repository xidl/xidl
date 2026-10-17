import shutil
import subprocess
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
        [str(Path.cwd() / "target/debug/xidlc"), "gen", "--help"], cwd=Path.cwd()
    )
    assert "typescript-rest-adapters" in help_result.stdout
    output = Path(context.lang_dir)
    context.ordinary_adapter_baseline = {
        path.name: path.read_bytes() for path in output.iterdir() if path.is_file()
    }
    command = [
        str(Path.cwd() / "target/debug/xidlc"),
        "gen",
        "--out-dir",
        str(output),
        "typescript-rest-adapters",
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


def prepare_application(context, script):
    root = Path.cwd()
    output = Path(context.lang_dir)
    boilerplate = root / "bdd/boilerplate/versioned_adapters/ts"
    package = (boilerplate / "package.json").read_text()
    for name in ["CLIENT", "CODEC", "SERVER"]:
        package = package.replace(
            "{{TS_XIDL_TYPESCRIPT_" + name + "_PATH}}",
            str(root / "typescript" / ("xidl-typescript-" + name.lower())),
        )
    (output / "package.json").write_text(package)
    shutil.copy(boilerplate / "tsconfig.json", output)
    shutil.copy(boilerplate / script, output)
    if script == "business.ts":
        shutil.copy(boilerplate / "business_http.ts", output)
    run(["pnpm", "install", "--no-frozen-lockfile"], cwd=output)
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


@then("the three-version TypeScript application enforces wire and business boundaries")
def real_business_journey(context):
    output = prepare_application(context, "business.ts")
    result = run(["pnpm", "exec", "tsx", "business.ts"], cwd=output)
    assert "PASS: three-version" in result.stdout
    missing_hook = output / "missing-hook.ts"
    missing_hook.write_text(
        'import type { VersionAdapters } from "./versioned_adapters_business.adapters";\n'
        "export const hooks: VersionAdapters = {};\n"
    )
    try:
        result = subprocess.run(
            ["pnpm", "exec", "tsc", "--noEmit"],
            cwd=output,
            capture_output=True,
            text=True,
        )
        assert result.returncode != 0, "missing business decisions must fail tsc"
        assert "parcelUpcastFromV1" in result.stdout + result.stderr
    finally:
        missing_hook.unlink()


@then("the single-version TypeScript application validates before dispatch")
def single_version_journey(context):
    output = prepare_application(context, "single.ts")
    result = run(["pnpm", "exec", "tsx", "single.ts"], cwd=output)
    assert "PASS: single-version" in result.stdout
