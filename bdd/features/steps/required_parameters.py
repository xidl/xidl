"""Exercise presence and explicit scalar values through generated SDKs."""

from pathlib import Path
import shutil
import subprocess

from behave import then


@then(
    'the required parameter SDK preserves explicit zero and absent values for "{language}"'
)
def required_parameter_sdk(context, language):
    directory = Path(context.lang_dir)
    base = f"http://127.0.0.1:{context.port}"
    if language == "rust":
        binary = Path.cwd() / "bdd/.temp/rust_target/debug/required-parameters"
        command = [str(binary), base]
    else:
        assert language == "ts"
        shutil.copy(
            Path.cwd() / "bdd/boilerplate/required_parameters/ts/sdk.ts", directory
        )
        subprocess.run(
            ["node", "node_modules/typescript/bin/tsc", "--noEmit"],
            cwd=directory,
            check=True,
            timeout=120,
        )
        command = ["node", "--import", "tsx", "sdk.ts", base]
    subprocess.run(command, cwd=directory, check=True, timeout=120)
