"""Use one generated SDK journey against each language's real HTTP server."""

from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile

from behave import then


class StreamSDKs:
    commands = {}

    @classmethod
    def register(cls, language, directory):
        directory = Path(directory)
        if language == "rust":
            command = [str(Path.cwd() / "bdd/.temp/rust_target/debug/stream-lifecycle")]
        elif language == "go":
            binary = directory / "stream-sdk"
            subprocess.run(
                ["go", "build", "-o", str(binary), "."], cwd=directory, check=True
            )
            command = [str(binary)]
        else:
            assert language == "ts"
            shutil.copy("bdd/boilerplate/stream_lifecycle/ts/sdk.ts", directory)
            subprocess.run(
                ["node", "node_modules/typescript/bin/tsc", "--noEmit"],
                cwd=directory,
                check=True,
                timeout=120,
            )
            command = ["node", "--import", "tsx", "sdk.ts"]
        cls.commands[language] = (directory, command)

    @classmethod
    def prepare(cls, language):
        root = Path.cwd()
        directory = Path(tempfile.mkdtemp(prefix="stream-sdk-", dir=root / "bdd/.temp"))
        cli = root / os.environ.get("CARGO_TARGET_DIR", "target") / "debug/xidlc"
        subprocess.run(
            [
                str(cli),
                "gen",
                "-o",
                str(directory),
                {"rust": "rust-axum", "go": "go-rest", "ts": "typescript-rest"}[
                    language
                ],
                "--client",
                "--server",
                str(root / "bdd/features/data/stream_lifecycle.idl"),
                str(root / "bdd/features/data/stream_versions.idl"),
            ],
            check=True,
        )
        source = root / "bdd/boilerplate/stream_lifecycle" / language
        shutil.copytree(source, directory, dirs_exist_ok=True)
        replacements = {
            "MODULE_NAME": "stream_lifecycle",
            "RUST_XIDL_RUST_AXUM_PATH": root / "crates/xidl-rust-axum",
            "GOLANG_XIDL_GO_REST_PATH": root / "golang/xidl-go-rest",
            "GOLANG_XIDL_GO_PATH": root / "golang/xidl-go",
            "GOLANG_XIDL_GO_CODEC_PATH": root / "golang/xidl-go-codec",
            "TS_XIDL_TYPESCRIPT_CODEC_PATH": root / "typescript/xidl-typescript-codec",
            "TS_XIDL_TYPESCRIPT_CLIENT_PATH": root
            / "typescript/xidl-typescript-client",
            "TS_XIDL_TYPESCRIPT_SERVER_PATH": root
            / "typescript/xidl-typescript-server",
        }
        for path in directory.rglob("*"):
            if not path.is_file():
                continue
            content = path.read_text()
            for key, value in replacements.items():
                content = content.replace("{{" + key + "}}", str(value))
            if language == "go" and path.suffix == ".go":
                content = re.sub(r"package \w+", "package main", content, count=1)
            path.write_text(content)
        if language == "rust":
            (directory / "src/lib.rs").write_text(
                "#[allow(non_camel_case_types)]\n"
                'pub mod versions { include!("../stream_versions.rs"); }\n'
            )
            env = os.environ.copy()
            env["CARGO_TARGET_DIR"] = str(root / "bdd/.temp/rust_target")
            subprocess.run(
                ["cargo", "build"], cwd=directory, env=env, check=True, timeout=180
            )
        elif language == "go":
            subprocess.run(["go", "mod", "tidy"], cwd=directory, check=True)
        else:
            subprocess.run(
                ["npm", "install", "--ignore-scripts"],
                cwd=directory,
                check=True,
                timeout=120,
            )
        cls.register(language, directory)


@then('generated stream SDKs preserve terminal business outcomes for "{language}"')
def stream_sdks(context, language):
    for client in ("rust", "go", "ts"):
        if client not in StreamSDKs.commands:
            StreamSDKs.prepare(client)
            context.add_cleanup(
                shutil.rmtree, StreamSDKs.commands[client][0], layer="testrun"
            )
        directory, command = StreamSDKs.commands[client]
        subprocess.run(
            command + [f"http://127.0.0.1:{context.port}"],
            cwd=directory,
            check=True,
            timeout=120,
        )
