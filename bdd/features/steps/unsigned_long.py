"""Validate emitted OpenAPI schemas with a JSON Schema implementation."""

import json
import os
from pathlib import Path
import subprocess

from behave import then, when
from jsonschema import Draft202012Validator


@when("I generate the unsigned integer OpenAPI contract")
def generate_unsigned(context):
    command = [
        "cargo",
        "run",
        "-p",
        "xidlc",
        "--features",
        "cli",
        "--",
        "gen",
        "-o",
        context.lang_dir,
        "openapi",
        context.idl_file,
    ]
    result = subprocess.run(command, cwd=os.getcwd(), capture_output=True, text=True)
    assert result.returncode == 0, result.stderr


@then(
    "its uint32 schemas accept the full unsigned range without changing other integers"
)
def validate_unsigned(context):
    files = list(Path(context.lang_dir).glob("*.json"))
    assert len(files) == 1, files
    document = json.loads(files[0].read_text())
    fields = document["components"]["schemas"]["Counters"]["properties"]
    path = document["paths"]["/counters/{id}"]["get"]["parameters"][0]["schema"]
    for schema in [
        fields["legacy"],
        fields["modern"],
        fields["samples"]["items"],
        path,
    ]:
        assert schema == {
            "type": "integer",
            "format": "int64",
            "minimum": 0,
            "maximum": 4294967295,
        }
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        for value in [0, 2147483647, 2147483648, 4294967295]:
            assert validator.is_valid(value), value
        for value in [-1, 4294967296, 1.5, "4294967295", None, True]:
            assert not validator.is_valid(value), value
    assert fields["small"] == {"type": "integer", "format": "int32", "minimum": 0}
    assert fields["signed_count"] == {"type": "integer", "format": "int32"}
    assert fields["wide"] == {"type": "integer", "format": "int64", "minimum": 0}
