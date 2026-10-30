"""Use one generated SDK journey against each language's real HTTP server."""

from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile


class GeneratedSDKs:
    commands = {}

    def __init__(self, fixture, companions=()):
        self.fixture = fixture
        self.companions = companions

    def register(self, language, directory):
        directory = Path(directory)
        if language == "rust":
            command = [
                str(
                    Path.cwd()
                    / "bdd/.temp/rust_target/debug"
                    / self.fixture.replace("_", "-")
                )
            ]
        elif language == "go":
            binary = directory / "stream-sdk"
            subprocess.run(
                ["go", "build", "-o", str(binary), "."], cwd=directory, check=True
            )
            command = [str(binary)]
        else:
            assert language == "ts"
            shutil.copy(Path("bdd/boilerplate") / self.fixture / "ts/sdk.ts", directory)
            subprocess.run(
                ["node", "node_modules/typescript/bin/tsc", "--noEmit"],
                cwd=directory,
                check=True,
                timeout=120,
            )
            command = ["node", "--import", "tsx", "sdk.ts"]
        self.commands[(self.fixture, language)] = (directory, command)

    def prepare(self, language):
        root = Path.cwd()
        directory = Path(
            tempfile.mkdtemp(prefix=f"{self.fixture}-sdk-", dir=root / "bdd/.temp")
        )
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
                str(root / "bdd/features/data" / f"{self.fixture}.idl"),
                *[str(root / "bdd/features/data" / name) for name in self.companions],
            ],
            check=True,
        )
        source = root / "bdd/boilerplate" / self.fixture / language
        shutil.copytree(source, directory, dirs_exist_ok=True)
        replacements = {
            "MODULE_NAME": self.fixture,
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
        self.register(language, directory)

    def run(self, context):
        for client in ("rust", "go", "ts"):
            key = (self.fixture, client)
            if key not in self.commands:
                self.prepare(client)
                context.add_cleanup(
                    shutil.rmtree, self.commands[key][0], layer="testrun"
                )
            directory, command = self.commands[key]
            subprocess.run(
                command + [f"http://127.0.0.1:{context.port}"],
                cwd=directory,
                check=True,
                timeout=120,
            )
