"""Exercise source declarations through the compiler and OpenRPC projection."""

import json
from pathlib import Path
import subprocess
import tempfile

from behave import given, then, when

from lint import compiler


@given('the source interface contract "{path}"')
def source_contract(context, path):
    context.interface_source = Path(path).resolve()
    context.temp_dir = tempfile.mkdtemp(prefix="xidl-source-interfaces-")


@when("I generate its OpenRPC documents twice")
def generate_documents(context):
    output = Path(context.temp_dir)
    command = [
        compiler(),
        "gen",
        "--out-dir",
        str(output),
        "openrpc",
        str(context.interface_source),
    ]
    previous = None
    for _ in range(2):
        result = subprocess.run(command, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, result.stdout + result.stderr
        documents = {p.name: p.read_bytes() for p in output.glob("*.json")}
        assert documents, result.stdout
        if previous is not None:
            assert documents == previous, "generation must preserve contract bytes"
        previous = documents
    context.interface_documents = [json.loads(value) for value in previous.values()]


@then("imported model names preserve every declared field and union branch")
def imported_schemas(context):
    [document] = context.interface_documents
    schemas = document["components"]["schemas"]
    assert set(schemas) == {
        "audit.Dispatch",
        "audit.Envelope",
        "dds.rpc.RequestHeader",
        "dds.rpc.ReplyHeader",
        "dds.rpc.UnusedMember",
        "dds.rpc.UnknownOperation",
    }
    envelope = schemas["audit.Envelope"]
    assert set(envelope["required"]) == {"request", "receipt", "note"}
    for field, model in [
        ("request", "RequestHeader"),
        ("receipt", "ReplyHeader"),
        ("note", "UnusedMember"),
    ]:
        assert envelope["properties"][field] == {
            "$ref": f"#/components/schemas/dds.rpc.{model}"
        }
    branches = schemas["audit.Dispatch"]["oneOf"]
    assert len(branches) == 2
    assert branches[1]["properties"]["rejected"] == {
        "$ref": "#/components/schemas/dds.rpc.UnknownOperation"
    }


@then("empty calls, parameter directions and attributes preserve their contracts")
def operation_contracts(context):
    [document] = context.interface_documents
    methods = {method["name"]: method for method in document["methods"]}
    assert set(methods) == {
        "Ledger.ping",
        "Ledger.revise",
        "Ledger.inspect",
        "Ledger.get_attribute_ready",
        "Ledger.get_attribute_channel",
        "Ledger.set_attribute_channel",
    }
    assert methods["Ledger.ping"]["params"] == []
    assert methods["Ledger.ping"]["result"]["schema"]["properties"] == {}
    revise = methods["Ledger.revise"]
    assert [param["name"] for param in revise["params"]] == ["envelope", "revision"]
    assert set(revise["result"]["schema"]["required"]) == {
        "return",
        "receipt",
        "revision",
    }


@then("the versioned OpenRPC methods match their declared lifetimes")
def versioned_contracts(context):
    documents = context.interface_documents
    assert len(documents) == 2
    for document in documents:
        methods = {method["name"] for method in document["methods"]}
        version = document["info"]["version"]
        module, operation = {"1.0": ("v1", "legacy"), "2.0": ("v2", "total")}[version]
        assert methods == {f"{module}.Calc.add", f"{module}.Calc.{operation}"}
        assert set(document["components"]["schemas"]) == {f"{module}.Sum"}
