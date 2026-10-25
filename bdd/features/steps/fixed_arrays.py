from pathlib import Path

from behave import then

from version_adapters import run


@then("the generated fixed-array validators accept only the declared stock grid")
def fixed_array_validators(context):
    output = Path(context.lang_dir)
    script = Path("bdd/boilerplate/wire_contract/ts/arrays.ts").read_text()
    (output / "arrays.ts").write_text(script)
    result = run(
        ["npx", "tsx", "arrays.ts", f"http://127.0.0.1:{context.port}"],
        cwd=output,
    )
    assert "PASS: stock grid" in result.stdout
