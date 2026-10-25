"""Split address libraries through version routers, chains and selected SDKs."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from behave import given, then, when
import requests

from server_process import start_server_process
from version_adapters import prepare_application


def run(command, directory, env=None):
    result = subprocess.run(
        command, cwd=directory, env=env, capture_output=True, text=True, timeout=600
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


@given('the inherited address application in "{mode}" mode')
def inherited_application(context, mode):
    assert mode in ("chain", "route")
    context.execute_steps(
        'Given a REST IDL file "bdd/features/data/versioned_libraries/service/xidl.toml"'
    )
    context.inherited_mode = mode


@when("I start the inherited address application with independently selected SDKs")
def inherited_generate(context):
    root = Path.cwd()
    output = Path(context.temp_dir)
    rust = output / "rust"
    typescript = output / "ts"
    cli = root / "target/debug/xidlc"
    libraries = root / "bdd/features/data/versioned_libraries"
    run(
        [str(cli), "lint", "--deny", "warnings", *map(str, libraries.rglob("*.idl"))],
        output,
    )

    def generate(target, folder, version=None, *options):
        command = [str(cli), "gen", "-o", str(folder)]
        if version is not None:
            command += ["--api-version", version]
        # Resolve paths from each manifest, independently of the working directory.
        run([*command, target, *options, context.idl_file], output)

    generate("rust-axum", rust, None, "--client", "--server")
    generate("rust-axum-adapters", rust)
    generate("openapi", output / "openapi")
    run(
        [
            str(cli),
            "gen",
            "-o",
            str(typescript / "imported"),
            "typescript-rest",
            "--client",
            "--server",
            str(libraries / "readers/xidl.toml"),
        ],
        output,
    )
    for version in ("v1", "v2", "v3"):
        generate("rust-axum", rust / "selected" / version, version, "--client")
        generate("typescript-rest", typescript / version, version, "--client")
        for path in (
            rust / "selected" / version / "versioned_inheritance.rs",
            typescript / version / "versioned_inheritance.client.ts",
        ):
            selected = path.read_text()
            for excluded in {"v1", "v2", "v3"} - {version}:
                assert f"pub mod {excluded} " not in selected
                assert f"export namespace {excluded} " not in selected
    boilerplate = root / "bdd/boilerplate/versioned_inheritance"
    (rust / "src").mkdir()
    for source in (boilerplate / "rust").glob("*.rs"):
        shutil.copy(source, rust / "src" / source.name)
    (rust / "Cargo.toml").write_text(
        (boilerplate / "rust/Cargo.toml")
        .read_text()
        .replace("{{RUST_XIDL_RUST_AXUM_PATH}}", str(root / "crates/xidl-rust-axum"))
    )
    package = (boilerplate / "ts/package.json").read_text()
    for name in ("client", "codec", "server"):
        package = package.replace(
            "{{TS_XIDL_TYPESCRIPT_" + name.upper() + "_PATH}}",
            str(root / "typescript" / f"xidl-typescript-{name}"),
        )
    (typescript / "package.json").write_text(package)
    shutil.copy(boilerplate / "ts/tsconfig.json", typescript)
    shutil.copy(boilerplate / "ts/sdk.ts", typescript)
    shutil.copy(boilerplate / "ts/imports.ts", typescript)
    run(["pnpm", "install", "--ignore-scripts", "--no-frozen-lockfile"], typescript)
    run(["pnpm", "exec", "tsc", "--noEmit"], typescript)
    env = os.environ.copy()
    target = root / "bdd/.temp/rust_target"
    env.update(CARGO_TARGET_DIR=str(target), PORT=str(context.port))
    run(["cargo", "build", "--quiet"], rust, env)
    context.inherited_binary = target / "debug/versioned-inheritance"
    context.inherited_ts = typescript
    context.inherited_base = f"http://127.0.0.1:{context.port}"
    context.port_guard.close()
    context.port_guard = None
    log = output / "inherited-server.log"
    with log.open("w") as stream:
        context.server_process = start_server_process(
            [str(context.inherited_binary), context.inherited_mode],
            cwd=rust,
            env=env,
            stdout=stream,
            stderr=subprocess.STDOUT,
        )
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        assert context.server_process.poll() is None, log.read_text()
        try:
            if (
                requests.get(context.inherited_base + "/ready", timeout=0.5).status_code
                == 204
            ):
                return
        except requests.ConnectionError:
            pass
        time.sleep(0.05)
    raise AssertionError(
        "inherited application did not become ready\n" + log.read_text()
    )


@then("the shared inherited address Hurl journey succeeds")
def inherited_hurl(context):
    context.inherited_report = Path(context.temp_dir) / "hurl-report"
    run(
        [
            "hurl",
            "--test",
            "--max-time",
            "5",
            "--report-json",
            str(context.inherited_report),
            "--variable",
            f"base_url={context.inherited_base}",
            "bdd/features/data/versioned_inheritance.hurl",
        ],
        Path.cwd(),
    )


@then("inherited address responses match their generated OpenAPI contracts")
def inherited_openapi(context):
    from jsonschema import Draft202012Validator

    documents = {
        document["info"]["version"]: document
        for path in (Path(context.temp_dir) / "openapi").glob("*.json")
        for document in [json.loads(path.read_text())]
    }
    assert set(documents) == {"2026-01-01", "2026-04-01", "2026-07-01"}
    report = json.loads((context.inherited_report / "report.json").read_text())
    checked_recursive_contracts = set()
    for result in report:
        for entry in result["entries"]:
            for call in entry["calls"]:
                response = call["response"]
                headers = {
                    header["name"].lower(): header["value"]
                    for header in response["headers"]
                }
                # /audit is test instrumentation, outside the API contract.
                if requests.utils.urlparse(call["request"]["url"]).path == "/audit":
                    assert response["status"] == 200
                    continue
                request_headers = {
                    header["name"].lower(): header["value"]
                    for header in call["request"]["headers"]
                }
                version = request_headers.get("x-address-version", "2026-07-01")
                assert headers.get("x-address-version") == version
                document = documents[version]
                operation = document["paths"]["/tenants/{tenant}/addresses/{id}"][
                    call["request"]["method"].lower()
                ]
                media_type = headers["content-type"].split(";", 1)[0].strip()
                contract = operation["responses"][str(response["status"])]["content"][
                    media_type
                ]["schema"]
                body = json.loads(
                    (context.inherited_report / response["body"]).read_text()
                )
                validator = Draft202012Validator(
                    {**contract, "components": document["components"]}
                )
                validator.validate(body)
                if (
                    response["status"] == 200
                    and version not in checked_recursive_contracts
                ):
                    malformed = json.loads(json.dumps(body))
                    malformed["parent"]["parent"]["checkpoint"] = 42
                    assert not validator.is_valid(malformed), (
                        f"{version} OpenAPI must validate recursive checkpoint types"
                    )
                    for coordinates in ([], [1], [1, 2, 3], [1, {}]):
                        malformed = json.loads(json.dumps(body))
                        malformed["coordinates"] = coordinates
                        assert not validator.is_valid(malformed), (
                            f"{version} OpenAPI must preserve coordinate arity and type"
                        )
                    checked_recursive_contracts.add(version)
    assert checked_recursive_contracts == set(documents)


@then("the independent address SDKs preserve inherited history")
def inherited_sdks(context):
    assert "PASS: independent Rust" in run(
        [str(context.inherited_binary), "sdk", context.inherited_base],
        Path(context.temp_dir),
    )
    assert "PASS: independent TypeScript" in run(
        ["pnpm", "exec", "tsx", "sdk.ts", context.inherited_base], context.inherited_ts
    )


@then("inherited request validation preserves the documented mode boundary")
def inherited_mode_boundary(context):
    base = context.inherited_base
    before = requests.get(base + "/audit", timeout=5).json()
    # Root alias identity and a deep recursive leaf are checked before mutation.
    for version, patch in [
        ("2026-01-01", {"deliveryReference": 42}),
        ("2026-01-01", {"coordinates": [1], "deliveryReference": "must-not-persist"}),
        ("2026-04-01", {"coordinates": [], "street": "must-not-persist"}),
        ("2026-07-01", {"coordinates": [1, 2, 3], "signatureRequired": False}),
        ("2026-07-01", {"coordinates": [1, {}], "signatureRequired": False}),
        (
            "2026-01-01",
            {
                "parent": {"parent": {"checkpoint": 42}},
                "deliveryReference": "must-not-persist",
            },
        ),
        (
            "2026-07-01",
            {"parent": {"parent": True}, "signatureRequired": False},
        ),
    ]:
        response = requests.patch(
            base + "/tenants/acme/addresses/a1",
            json=patch,
            headers={"x-address-version": version, "authorization": "Bearer test-acme"},
            timeout=5,
        )
        assert response.status_code == 400, response.text
        assert requests.get(base + "/audit", timeout=5).json() == before
    # Ordinary serde routers currently ignore unknown fields. The adapter wire
    # decoder rejects them before hooks. Neither may persist the hidden value.
    for version, patch in [
        ("2026-01-01", {"signatureRequired": True}),
        ("2026-07-01", {"deliveryReference": "injected", "signatureRequired": False}),
    ]:
        response = requests.patch(
            base + "/tenants/acme/addresses/a1",
            json=patch,
            headers={"x-address-version": version, "authorization": "Bearer test-acme"},
            timeout=5,
        )
        assert response.status_code == (
            400 if context.inherited_mode == "chain" else 200
        ), response.text
    assert requests.get(base + "/audit", timeout=5).json() == before


@then("the TypeScript inherited request journey preserves Head and stored facts")
def inherited_typescript(context):
    output = prepare_application(context, "inheritance.ts", "versioned_inheritance")
    assert "PASS: TypeScript inherited" in run(
        ["pnpm", "exec", "tsx", "inheritance.ts"], output
    )
