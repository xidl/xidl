"""Validate recorded HTTP responses using only the generated OpenAPI document."""

import json
from pathlib import Path
import re
from urllib.parse import urlparse

from jsonschema import Draft202012Validator


class OpenAPIResponses:
    def __init__(self, report):
        self.report = Path(report)

    def calls(self):
        for result in json.loads((self.report / "report.json").read_text()):
            for entry in result["entries"]:
                yield from entry["calls"]

    @staticmethod
    def validator(schema, document):
        return Draft202012Validator(
            {**schema, "components": document.get("components", {})}
        )

    @staticmethod
    def response_contract(responses, status):
        code = str(status)
        contract = responses.get(
            code, responses.get(code[0] + "XX", responses.get("default"))
        )
        assert contract is not None, (code, responses.keys())
        return contract

    def validate(self, call, document):
        request, response = call["request"], call["response"]
        path = urlparse(request["url"]).path
        method = request["method"].lower()
        exact = document["paths"].get(path)
        matches = (
            [exact]
            if exact and method in exact
            else [
                item
                for template, item in document["paths"].items()
                if method in item
                and re.fullmatch(
                    "/".join(
                        "[^/]+" if segment.startswith("{") else re.escape(segment)
                        for segment in template.split("/")
                    ),
                    path,
                )
            ]
        )
        assert len(matches) == 1, (path, matches)
        operation = matches[0][request["method"].lower()]
        responses = operation["responses"]
        status = str(response["status"])
        contract = self.response_contract(responses, status)
        headers = {h["name"].lower(): h["value"] for h in response["headers"]}
        for name, header in contract.get("headers", {}).items():
            if name.lower() in headers:
                raw = headers[name.lower()]
                schema = header["schema"]
                value = (
                    json.loads(raw)
                    if schema.get("type")
                    in ("integer", "number", "boolean", "array", "object")
                    else raw
                )
                self.validator(schema, document).validate(value)
        body = (
            (self.report / response["body"]).read_bytes()
            if response.get("body")
            else b""
        )
        if not contract.get("content"):
            assert not body, (path, status, body)
            return None
        mime = headers["content-type"].split(";", 1)[0].strip()
        representation = contract["content"][mime]
        if mime == "text/event-stream":
            self.validate_stream(
                body.decode(), representation["itemSchema"], operation, document
            )
            return None
        assert "schema" in representation, (path, mime, representation)
        schema = representation["schema"]
        value = (
            json.loads(body)
            if mime.endswith("json") or mime.endswith("+json")
            else body.decode()
        )
        self.validator(schema, document).validate(value)
        return value

    def validate_stream(self, body, item_schema, operation, document):
        terminal = False
        for block in re.split(r"\r?\n\r?\n", body)[:-1]:
            data, event = [], "message"
            for line in block.splitlines():
                if line.startswith("event:"):
                    event = line[6:].strip() or "message"
                elif line.startswith("data:"):
                    data.append(line[5:].lstrip(" "))
            if not data:
                continue
            assert not terminal, "stream emitted a frame after its terminal outcome"
            value = json.loads("\n".join(data))
            if event in ("next", "message"):
                self.validator(item_schema, document).validate(value)
            elif event == "error":
                response = self.response_contract(
                    operation["responses"], value["status"]
                )
                if response.get("content"):
                    schema = response["content"]["application/json"]["schema"]
                    self.validator(schema, document).validate(value["body"])
                else:
                    assert value["body"] is None
                terminal = True
            elif event == "complete":
                assert value == {}, "complete frame must have an empty object payload"
                terminal = True
            else:
                assert event == "heartbeat", event
        assert terminal, "structured stream ended without a terminal outcome"
