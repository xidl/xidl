"""Use the generated clients after the shared raw HTTP collection journey."""

from pathlib import Path
import shutil
import subprocess

from behave import then


@then('collection SDKs preserve zero singleton and repeated inputs for "{language}"')
def collection_sdks(context, language):
    directory = Path(context.lang_dir)
    base = f"http://127.0.0.1:{context.port}"
    if language == "rust":
        command = [
            str(Path.cwd() / "bdd/.temp/rust_target/debug/http-collections"),
            base,
        ]
    else:
        assert language == "ts"
        shutil.copy(
            Path.cwd() / "bdd/boilerplate/http_collections/ts/sdk.ts", directory
        )
        subprocess.run(
            ["node", "node_modules/typescript/bin/tsc", "--noEmit"],
            cwd=directory,
            check=True,
            timeout=120,
        )
        command = ["node", "--import", "tsx", "sdk.ts", base]
    subprocess.run(command, cwd=directory, check=True, timeout=120)
