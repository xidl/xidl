"""Generate real SDKs and exercise their version metadata over local HTTP/WS."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from behave import given, then, when


def run(command, directory, env=None):
    result = subprocess.run(
        command,
        cwd=directory,
        env=env,
        capture_output=True,
        text=True,
        timeout=600,
    )
    assert result.returncode == 0, (
        f"{command} failed ({result.returncode})\n{result.stdout}\n{result.stderr}"
    )
    return result.stdout


@given('a version parameter contract carried in "{location}"')
def version_contract(context, location):
    context.version_location = location
    context.version_name = {
        "header": "X-Version",
        "query": "api_version",
        "cookie": "api%v",
    }[location]
    context.version_value = "2026-07" if location == "header" else "版本 +/%;="
    root = Path.cwd()
    (root / "bdd/.temp").mkdir(exist_ok=True)
    context.temp_dir = tempfile.mkdtemp(dir=root / "bdd/.temp")
    context.version_root = Path(context.temp_dir)
    source = (root / "bdd/features/data/version_parameter.idl").read_text()
    source = source.replace("LOCATION", location)
    source = source.replace("VERSION_NAME", context.version_name)
    source = source.replace("VERSION_VALUE", context.version_value)
    context.version_idl = context.version_root / "version_parameter.idl"
    context.version_idl.write_text(source)


@when("I generate both SDKs and OpenAPI for the version parameter contract")
def version_generate(context):
    for target, folder in [
        ("rust-axum", "rust"),
        ("typescript-rest", "ts"),
        ("go-rest", "go"),
        ("openapi", "openapi"),
    ]:
        args = [
            "cargo",
            "run",
            "-p",
            "xidlc",
            "--features",
            "cli,fmt",
            "--",
            "gen",
            "-o",
            str(context.version_root / folder),
            target,
        ]
        if target != "openapi":
            args += ["--client", "--server"]
        run(args + [str(context.version_idl)], Path.cwd())


@then("every operation describes its own optional version parameter")
def version_openapi(context):
    files = sorted((context.version_root / "openapi").glob("*.json"))
    assert len(files) == 2, files
    for file, expected in zip(files, ["2026-04", context.version_value]):
        document = json.loads(file.read_text())
        assert document["info"]["version"] == expected
        count = 0
        for path in document["paths"].values():
            for method, operation in path.items():
                if method not in (
                    "get",
                    "post",
                    "put",
                    "patch",
                    "delete",
                    "head",
                    "options",
                ):
                    continue
                count += 1
                params = [
                    p
                    for p in operation.get("parameters", [])
                    if p["in"] == context.version_location
                    and p["name"] == context.version_name
                ]
                assert len(params) == 1, operation
                assert params[0]["required"] is False
                assert params[0]["schema"]["enum"] == [expected]
                for response in operation["responses"].values():
                    headers = response.get("headers", {})
                    if context.version_location == "header":
                        assert headers[context.version_name]["schema"]["enum"] == [
                            expected
                        ]
                    else:
                        assert context.version_name not in headers
        assert count == 9, count


def prepare(context, language):
    source = Path.cwd() / "bdd/boilerplate/version_parameter" / language
    destination = context.version_root / language
    shutil.copytree(source, destination, dirs_exist_ok=True)
    env = os.environ.copy()
    env.update(
        VERSION_LOCATION=context.version_location,
        VERSION_NAME=context.version_name,
        VERSION_VALUE=context.version_value,
        GOCACHE="off",
        GOTOOLCHAIN="local",
    )
    return destination, env


@then("the generated Rust SDK exchanges HTTP and WebSocket messages with both versions")
def version_rust(context):
    destination, env = prepare(context, "rust")
    manifest = destination / "Cargo.toml"
    manifest.write_text(
        manifest.read_text().replace(
            "{{RUST_XIDL_RUST_AXUM_PATH}}", str(Path.cwd() / "crates/xidl-rust-axum")
        )
    )
    shutil.move(destination / "version_parameter.rs", destination / "src/gen.rs")
    run(["cargo", "run", "--quiet"], destination, env)


@then(
    "the generated TypeScript SDK exchanges HTTP and WebSocket messages with both versions"
)
def version_typescript(context):
    destination, env = prepare(context, "ts")
    manifest = destination / "package.json"
    text = manifest.read_text()
    for package in ["client", "codec", "server"]:
        text = text.replace(
            f"{{{{TS_XIDL_TYPESCRIPT_{package.upper()}_PATH}}}}",
            str(Path.cwd() / f"typescript/xidl-typescript-{package}"),
        )
    manifest.write_text(text)
    run(
        ["pnpm", "install", "--no-frozen-lockfile"],
        destination,
        env,
    )
    run(["pnpm", "exec", "tsc", "--noEmit"], destination, env)
    run(["pnpm", "exec", "tsx", "test.ts"], destination, env)


@then("the generated Go SDK exchanges HTTP messages with both versions")
def version_go(context):
    destination = context.version_root / "go"
    destination.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(
        VERSION_LOCATION=context.version_location,
        VERSION_NAME=context.version_name,
        VERSION_VALUE=context.version_value,
        GOCACHE=str(context.version_root / "go-cache"),
    )
    module = destination / "go.mod"
    module.write_text(
        """module example.com/version_parameter

go 1.23

require github.com/xidl/xidl/golang/xidl-go-rest v0.0.0

replace github.com/xidl/xidl/golang/xidl-go-rest => {{GOLANG_XIDL_GO_REST_PATH}}
replace github.com/xidl/xidl/golang/xidl-go-codec => {{GOLANG_XIDL_GO_CODEC_PATH}}
""".replace(
            "{{GOLANG_XIDL_GO_REST_PATH}}", str(Path.cwd() / "golang/xidl-go-rest")
        ).replace(
            "{{GOLANG_XIDL_GO_CODEC_PATH}}", str(Path.cwd() / "golang/xidl-go-codec")
        )
    )
    test = destination / "version_parameter_test.go"
    test.write_text(
        """package version_parameter

import (
    "context"
    "net/http"
    "net/http/httptest"
    "net/url"
    "testing"

    xidlgohttp "github.com/xidl/xidl/golang/xidl-go-rest"
    "github.com/gin-gonic/gin"
)

type healthV1 struct{}
func (healthV1) Health(context.Context, *V1BusinessHealthHealthRequest) (*V1BusinessHealthHealthResponse, error) {
    return &V1BusinessHealthHealthResponse{Return: "healthy"}, nil
}
type healthV2 struct{}
func (healthV2) Health(context.Context, *V2BusinessHealthHealthRequest) (*V2BusinessHealthHealthResponse, error) {
    return &V2BusinessHealthHealthResponse{Return: "healthy"}, nil
}

func exercise(t *testing.T, expected string, register func(*gin.Engine), call func(string) error) {
    t.Helper()
    gin.SetMode(gin.TestMode)
    router := gin.New()
    var request *http.Request
    var response http.Header
    router.Use(func(c *gin.Context) {
        c.Next()
        request = c.Request
        response = c.Writer.Header().Clone()
    })
    register(router)
    server := httptest.NewServer(router)
    defer server.Close()
    if err := call(server.URL); err != nil { t.Fatal(err) }
    if request == nil { t.Fatal("server did not observe request") }
    switch {{VERSION_LOCATION|quote}} {
    case "header":
        if got := request.Header.Get({{VERSION_NAME|quote}}); got != expected { t.Fatalf("version header = %q", got) }
        if got := response.Get({{VERSION_NAME|quote}}); got != expected { t.Fatalf("response version header = %q", got) }
    case "query":
        if got := request.URL.Query().Get({{VERSION_NAME|quote}}); got != expected { t.Fatalf("version query = %q", got) }
        if response.Get({{VERSION_NAME|quote}}) != "" { t.Fatal("query version echoed in response") }
    case "cookie":
        cookie, err := request.Cookie({{VERSION_NAME|quote}}); if err != nil { t.Fatal(err) }
        got, err := url.QueryUnescape(cookie.Value); if err != nil { t.Fatal(err) }
        if got != expected { t.Fatalf("version cookie = %q", got) }
        if response.Get({{VERSION_NAME|quote}}) != "" { t.Fatal("cookie version echoed in response") }
    }
}

func TestGeneratedVersionClients(t *testing.T) {
    exercise(t, "2026-04", func(r *gin.Engine) { RegisterV1BusinessHealthHandler(r, healthV1{}) }, func(base string) error {
        client := NewV1BusinessHealthClient(base, http.DefaultClient, xidlgohttp.ClientAuth{})
        response, err := client.Health(context.Background(), &V1BusinessHealthHealthRequest{})
        if err != nil { return err }; if response.Return != "healthy" { t.Fatalf("health = %q", response.Return) }; return nil
    })
    exercise(t, {{VERSION_VALUE|quote}}, func(r *gin.Engine) { RegisterV2BusinessHealthHandler(r, healthV2{}) }, func(base string) error {
        client := NewV2BusinessHealthClient(base, http.DefaultClient, xidlgohttp.ClientAuth{})
        response, err := client.Health(context.Background(), &V2BusinessHealthHealthRequest{})
        if err != nil { return err }; if response.Return != "healthy" { t.Fatalf("health = %q", response.Return) }; return nil
    })
}
""".replace("{{VERSION_LOCATION|quote}}", json.dumps(context.version_location))
        .replace("{{VERSION_NAME|quote}}", json.dumps(context.version_name))
        .replace("{{VERSION_VALUE|quote}}", json.dumps(context.version_value))
    )
    run(["go", "mod", "tidy"], destination, env)
    for name in ("xidl-go-rest", "xidl-go-codec"):
        resolved = json.loads(
            run(
                ["go", "list", "-m", "-json", f"github.com/xidl/xidl/golang/{name}"],
                destination,
                env,
            )
        )
        assert (
            Path(resolved["Dir"]).resolve() == (Path.cwd() / "golang" / name).resolve()
        ), f"{name} must resolve to the current checkout, got {resolved['Dir']}"
    run(["go", "test", "./..."], destination, env)
