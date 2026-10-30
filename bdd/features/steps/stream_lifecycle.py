"""Run the same generated stream SDKs against every language's server."""

from behave import then
from generated_sdks import GeneratedSDKs


@then('generated stream SDKs preserve terminal business outcomes for "{language}"')
def stream_sdks(context, language):
    assert language in ("rust", "go", "ts")
    GeneratedSDKs("stream_lifecycle", ("stream_versions.idl",)).run(context)
