from behave import then

from version_adapters import prepare_application, prepare_rust_application, run


@then("the Rust bounded adapter journey preserves limits before dispatch")
def rust_bounded_adapter_journey(context):
    output = prepare_rust_application(
        context, "main.rs", boilerplate_name="versioned_bounds"
    )
    result = run(["cargo", "run", "--quiet"], cwd=output)
    assert "PASS: bounded projections" in result.stdout


@then("the TypeScript bounded adapter journey preserves limits before dispatch")
def typescript_bounded_adapter_journey(context):
    output = prepare_application(
        context, "bounds.ts", boilerplate_name="versioned_bounds"
    )
    result = run(["pnpm", "exec", "tsx", "bounds.ts"], cwd=output)
    assert "PASS: bounded projections" in result.stdout
