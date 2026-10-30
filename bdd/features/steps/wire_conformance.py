"""Compose existing Hurl reports, generated contracts and generated SDKs."""

import json
import os
from pathlib import Path
import subprocess

from behave import then
from generated_sdks import GeneratedSDKs
from openapi_responses import OpenAPIResponses


@then("every Hurl response matches its generated OpenAPI contract")
def openapi_responses(context):
    root = Path.cwd()
    output = Path(context.temp_dir) / "openapi"
    cli = root / os.environ.get("CARGO_TARGET_DIR", "target") / "debug/xidlc"
    subprocess.run(
        [str(cli), "gen", "-o", str(output), "openapi", *context.idl_files],
        check=True,
        timeout=120,
    )
    documents = [json.loads(path.read_text()) for path in output.glob("*.json")]
    documents = [document for document in documents if document.get("paths")]
    assert len(documents) == 1, (
        "ordinary wire probe must have one generated HTTP contract"
    )
    for report in context.hurl_reports:
        responses = OpenAPIResponses(report)
        for call in responses.calls():
            responses.validate(call, documents[0])


@then('generated clients use the same "{fixture}" wire contract')
def generated_clients(context, fixture):
    assert Path(context.idl_file).stem == fixture
    GeneratedSDKs(fixture).run(context)
