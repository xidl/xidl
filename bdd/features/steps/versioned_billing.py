"""Exercise generated version routers and chain adapters with one real ledger."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from behave import given, then, when
import requests

from server_process import start_server_process


def run(command, directory, env=None):
    result = subprocess.run(
        command, cwd=directory, env=env, text=True, capture_output=True, timeout=600
    )
    assert result.returncode == 0, (
        f"{command} failed ({result.returncode})\n{result.stdout}\n{result.stderr}"
    )
    return result.stdout


@given('the versioned billing application in "{mode}" mode')
def billing_contract(context, mode):
    assert mode in ("chain", "route")
    context.execute_steps(
        'Given a REST IDL file "bdd/features/data/versioned_billing.idl"'
    )
    context.billing_mode = mode


@when("I generate independently selected billing SDKs and start the application")
def billing_generate(context):
    root = Path.cwd()
    output = Path(context.temp_dir)
    rust = output / "rust"
    typescript = output / "ts"
    cli = root / "target/debug/xidlc"

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
        selected = (rust / "selected" / version / "versioned_billing.rs").read_text()
        assert f"pub mod {version}" in selected
        assert f"pub mod {'v2' if version == 'v1' else 'v1'}" not in selected
        generate(
            "typescript-rest",
            typescript / version,
            "--client",
            "--api-version",
            version,
        )

    boilerplate = root / "bdd/boilerplate/versioned_billing"
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
    run(["pnpm", "exec", "tsc", "--noEmit"], typescript)

    env = os.environ.copy()
    target = root / "bdd/.temp/rust_target"
    env.update(CARGO_TARGET_DIR=str(target), PORT=str(context.port))
    run(["cargo", "build", "--quiet"], rust, env)
    context.billing_binary = target / "debug/versioned-billing"
    context.billing_typescript = typescript
    context.billing_base = f"http://127.0.0.1:{context.port}"
    context.port_guard.close()
    context.port_guard = None
    log_path = output / "server.log"
    with log_path.open("w") as log:
        context.server_process = start_server_process(
            [str(context.billing_binary), context.billing_mode],
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
                requests.get(context.billing_base + "/ready", timeout=0.5).status_code
                == 204
            ):
                return
        except requests.ConnectionError:
            pass
        time.sleep(0.05)
    raise AssertionError(
        "billing application did not become ready\n" + log_path.read_text()
    )


@then("the shared billing Hurl journey preserves validation and committed payments")
def billing_hurl(context):
    context.billing_report = Path(context.temp_dir) / "hurl-report"
    run(
        [
            "hurl",
            "--test",
            "--max-time",
            "5",
            "--report-json",
            str(context.billing_report),
            "--variable",
            f"base_url={context.billing_base}",
            "bdd/features/data/versioned_billing.hurl",
        ],
        Path.cwd(),
    )


@then("every billing Hurl response matches its generated OpenAPI contract")
def billing_openapi(context):
    from jsonschema import Draft202012Validator

    documents = {
        document["info"]["version"]: document
        for path in (Path(context.temp_dir) / "openapi").glob("*.json")
        for document in [json.loads(path.read_text())]
    }
    assert set(documents) == {"2025-01-01", "2025-04-01"}
    report = json.loads((context.billing_report / "report.json").read_text())
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
                version = request_headers.get("x-billing-version", "2025-04-01")
                if version in documents:
                    assert headers.get("x-billing-version") == version
                else:
                    assert response["status"] == 400
                    assert "x-billing-version" not in headers
                    version = "2025-04-01"
                document = documents[version]
                path = requests.utils.urlparse(request["url"]).path
                if path.startswith("/invoices/"):
                    path = "/invoices/{id}"
                operation = document["paths"][path][request["method"].lower()]
                contract = operation["responses"][str(response["status"])]
                media_type = headers["content-type"].split(";", 1)[0].strip()
                schema = contract["content"][media_type]["schema"]
                for name, header in contract.get("headers", {}).items():
                    if name.lower() in headers:
                        Draft202012Validator(header["schema"]).validate(
                            headers[name.lower()]
                        )
                body = json.loads(
                    (context.billing_report / response["body"]).read_text()
                )
                validator = Draft202012Validator(
                    {**schema, "components": document.get("components", {})}
                )
                validator.validate(body)


@then("independent Rust and TypeScript billing SDKs use the same versioned endpoint")
def billing_sdks(context):
    stdout = run(
        [str(context.billing_binary), "sdk", context.billing_base],
        Path(context.temp_dir),
    )
    assert "PASS: independent Rust SDKs" in stdout
    stdout = run(
        ["pnpm", "exec", "tsx", "sdk.ts", context.billing_base],
        context.billing_typescript,
    )
    assert "PASS: independent TypeScript SDKs" in stdout
