import json
import os
import subprocess
import tempfile
from pathlib import Path

from behave import then


@then("the version map distinguishes replacements from surviving declarations")
def declaration_replacements(context):
    assert context.version_map_bytes == context.version_map_second_bytes
    document = json.loads(context.version_map_bytes)
    assert document["format"] == 2
    assert [version["id"] for version in document["versions"]] == ["v1", "v2", "v3"]
    first, second = document["steps"]
    for owner, name in [("Item", "label"), ("Failure", "reason"), ("Flags", "enabled")]:
        change = first["types"][owner]["members"][name]
        assert change["change"] == "changed"
        old, new = change["from"].copy(), change["to"].copy()
        assert old.pop("declaration_id") != new.pop("declaration_id")
        assert old == new
    assert "stable" not in first["types"]["Item"]["members"]
    operation = first["operations"]["Items.send"]
    assert operation["from"]["declaration_id"] == operation["to"]["declaration_id"]
    old = operation["from"]["parameters"]["query:token"]
    new = operation["to"]["parameters"]["query:token"]
    assert old["declaration_id"] != new["declaration_id"]
    split = first["operations"]["Items.split"]
    old, new = split["from"], split["to"]
    assert old["parameters"]["body:token"] == new["parameters"]["body:token"]
    assert (
        old["response_parameters"]["body:token"][0]["declaration_id"]
        != new["response_parameters"]["body:token"][0]["declaration_id"]
    )
    status = first["operations"]["Items.status"]["from"]["parameters"]
    assert (
        status["query:left"]["declaration_id"]
        != status["query:right"]["declaration_id"]
    )
    assert "Items.get_attribute_status" not in first["operations"]
    assert second == {"from": "v2", "to": "v3"}
    assert context.version_map_bytes.endswith(b"\n")


@then("source trivia and version selection preserve the complete declaration map")
def stable_declaration_map(context):
    source = Path(context.idl_file)
    with tempfile.TemporaryDirectory(prefix="xidl-map-declarations-") as temp:
        root = Path(temp)
        reformatted = root / source.name
        reformatted.write_text(
            source.read_text().replace(";", "; /* moved comment */\n")
        )
        command = [
            str(Path.cwd() / "target/debug/xidlc"),
            "gen",
            "--api-version",
            "v3",
            "--out-dir",
            str(root / "out"),
            "version-map",
            str(reformatted),
        ]
        result = subprocess.run(
            command, capture_output=True, text=True, cwd=os.getcwd()
        )
        assert result.returncode == 0, result.stderr
        actual = (root / "out" / f"{source.stem}.version-map.json").read_bytes()
        assert actual == context.version_map_bytes
