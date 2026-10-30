"""Run the same business journey through generated version routers and chains."""

from dataclasses import dataclass
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from behave import given, then, when
import requests

from openapi_responses import OpenAPIResponses
from server_process import start_server_process


@dataclass(frozen=True)
class Application:
    package: str
    header: str
    versions: tuple[str, str]


APPLICATIONS = {
    "billing": Application(
        "versioned_billing", "x-billing-version", ("2025-01-01", "2025-04-01")
    ),
    "catalog": Application(
        "versioned_catalog", "x-catalog-version", ("2026-07-01", "2026-10-01")
    ),
}


def run(command, directory, env=None):
    result = subprocess.run(
        command, cwd=directory, env=env, text=True, capture_output=True, timeout=600
    )
    assert result.returncode == 0, (
        f"{command} failed ({result.returncode})\n{result.stdout}\n{result.stderr}"
    )
    return result.stdout


@given('the versioned "{application}" application in "{mode}" mode')
def versioned_contract(context, application, mode):
    assert mode in ("chain", "route")
    context.versioned_app = APPLICATIONS[application]
    context.execute_steps(
        f'Given a REST IDL file "bdd/features/data/{context.versioned_app.package}.idl"'
    )
    context.versioned_mode = mode


@when("I generate independently selected SDKs and start the versioned application")
def versioned_generate(context):
    root = Path.cwd()
    output = Path(context.temp_dir)
    package_name = context.versioned_app.package
    rust = output / "rust"
    typescript = output / "ts"
    cli = root / os.environ.get("CARGO_TARGET_DIR", "target") / "debug/xidlc"

    def generate(target, folder, *options):
        run(
            [str(cli), "gen", "-o", str(folder), target, *options, context.idl_file],
            root,
        )

    generate("rust-axum", rust, "--client", "--server")
    generate("rust-axum-adapters", rust)
    generate("openapi", output / "openapi")
    for version in ("v1", "v2"):
        generate(
            "rust-axum",
            rust / "selected" / version,
            "--client",
            "--api-version",
            version,
        )
        selected = (rust / "selected" / version / f"{package_name}.rs").read_text()
        assert f"pub mod {version}" in selected
        assert f"pub mod {'v2' if version == 'v1' else 'v1'}" not in selected
        generate(
            "typescript-rest",
            typescript / version,
            "--client",
            "--api-version",
            version,
        )

    boilerplate = root / "bdd/boilerplate" / package_name
    shutil.copytree(boilerplate / "rust", rust, dirs_exist_ok=True)
    manifest = rust / "Cargo.toml"
    manifest.write_text(
        manifest.read_text().replace(
            "{{RUST_XIDL_RUST_AXUM_PATH}}", str(root / "crates/xidl-rust-axum")
        )
    )
    shutil.copy(boilerplate / "ts/sdk.ts", typescript)
    package = (root / "bdd/boilerplate/required_parameters/ts/package.json").read_text()
    for name in ("client", "codec", "server"):
        path = root / "typescript" / f"xidl-typescript-{name}"
        if not (path / "dist").exists():
            run(["pnpm", "install", "--ignore-scripts"], path)
            run(["pnpm", "build"], path)
        package = package.replace(
            "{{TS_XIDL_TYPESCRIPT_" + name.upper() + "_PATH}}", str(path)
        )
    (typescript / "package.json").write_text(package)
    shutil.copy(
        root / "bdd/boilerplate/required_parameters/ts/tsconfig.json", typescript
    )
    run(["pnpm", "install", "--ignore-scripts", "--no-frozen-lockfile"], typescript)
    run(["node", "node_modules/typescript/bin/tsc", "--noEmit"], typescript)

    env = os.environ.copy()
    target = root / "bdd/.temp/rust_target"
    env.update(CARGO_TARGET_DIR=str(target), PORT=str(context.port))
    run(["cargo", "build", "--quiet"], rust, env)
    context.versioned_binary = target / "debug" / package_name.replace("_", "-")
    context.versioned_typescript = typescript
    context.versioned_base = f"http://127.0.0.1:{context.port}"
    context.port_guard.close()
    context.port_guard = None
    log_path = output / "server.log"
    with log_path.open("w") as log:
        context.server_process = start_server_process(
            [str(context.versioned_binary), context.versioned_mode],
            cwd=rust,
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        assert context.server_process.poll() is None, log_path.read_text()
        try:
            if (
                requests.get(context.versioned_base + "/ready", timeout=0.5).status_code
                == 204
            ):
                return
        except requests.ConnectionError:
            pass
        time.sleep(0.05)
    raise AssertionError(
        f"{package_name} application did not become ready\n" + log_path.read_text()
    )


@then("the shared versioned Hurl journey succeeds")
def versioned_hurl(context):
    context.versioned_report = Path(context.temp_dir) / "hurl-report"
    run(
        [
            "hurl",
            "--test",
            "--max-time",
            "5",
            "--report-json",
            str(context.versioned_report),
            "--variable",
            f"base_url={context.versioned_base}",
            f"bdd/features/data/{context.versioned_app.package}.hurl",
        ],
        Path.cwd(),
    )


@then("every versioned Hurl response matches its generated OpenAPI contract")
def versioned_openapi(context):
    documents = {
        document["info"]["version"]: document
        for path in (Path(context.temp_dir) / "openapi").glob("*.json")
        for document in [json.loads(path.read_text())]
    }
    assert set(documents) == set(context.versioned_app.versions)
    report = json.loads((context.versioned_report / "report.json").read_text())
    # Hurl's report contains the actual response files; schemas come only from generation.
    for result in report:
        for entry in result["entries"]:
            for call in entry["calls"]:
                response = call["response"]
                headers = {
                    header["name"].lower(): header["value"]
                    for header in response["headers"]
                }
                request = call["request"]
                request_headers = {
                    header["name"].lower(): header["value"]
                    for header in request["headers"]
                }
                version_header = context.versioned_app.header
                version = request_headers.get(
                    version_header, context.versioned_app.versions[-1]
                )
                if version in documents:
                    assert headers.get(version_header) == version
                else:
                    assert response["status"] == 400
                    assert version_header not in headers
                    version = context.versioned_app.versions[-1]
                OpenAPIResponses(context.versioned_report).validate(
                    call, documents[version]
                )


@then("independent Rust and TypeScript SDKs use the same versioned endpoint")
def versioned_sdks(context):
    stdout = run(
        [str(context.versioned_binary), "sdk", context.versioned_base],
        Path(context.temp_dir),
    )
    assert "PASS: independent Rust SDKs" in stdout
    stdout = run(
        ["node", "--import", "tsx", "sdk.ts", context.versioned_base],
        context.versioned_typescript,
    )
    assert "PASS: independent TypeScript SDKs" in stdout
