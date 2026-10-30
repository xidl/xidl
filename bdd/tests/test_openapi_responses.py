import json
from pathlib import Path
import tempfile
import unittest

from jsonschema import ValidationError

from bdd.features.openapi_responses import OpenAPIResponses


class OpenAPIResponseTests(unittest.TestCase):
    def test_recorded_response_uses_exact_route_and_resolves_document_references(self):
        document = {
            "paths": {
                "/items/{id}": {"get": {"responses": {"200": {}}}},
                "/items/current": {
                    "get": {
                        "responses": {
                            "200": {
                                "headers": {"X-Count": {"schema": {"type": "integer"}}},
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/Item"}
                                    }
                                },
                            }
                        }
                    }
                },
            },
            "components": {
                "schemas": {
                    "Item": {
                        "type": "object",
                        "required": ["sku"],
                        "properties": {"sku": {"type": "string"}},
                    }
                }
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.bin"
            body.write_text(json.dumps({"sku": "雪"}))
            call = {
                "request": {"url": "http://localhost/items/current", "method": "GET"},
                "response": {
                    "status": 200,
                    "body": body.name,
                    "headers": [
                        {"name": "Content-Type", "value": "application/json"},
                        {"name": "X-Count", "value": "1"},
                    ],
                },
            }
            validator = OpenAPIResponses(directory)
            self.assertEqual(validator.validate(call, document), {"sku": "雪"})
            body.write_text('{"sku":1}')
            with self.assertRaises(ValidationError):
                validator.validate(call, document)

    def test_structured_stream_cannot_hide_truncation_or_an_invalid_error(self):
        validator = OpenAPIResponses("unused")
        item = {"type": "string"}
        operation = {
            "responses": {
                "409": {"content": {"application/json": {"schema": {"type": "string"}}}}
            }
        }
        next_frame = 'event: next\ndata: "雪"\n\n'
        complete = "event: complete\ndata: {}\n\n"
        error = 'event: error\ndata: {"status":409,"body":"locked"}\n\n'
        for body in [next_frame + complete, next_frame + error]:
            validator.validate_stream(body, item, operation, {})
        for body in [
            next_frame,
            next_frame + complete.rstrip(),
            complete + next_frame,
            "event: complete\ndata: 0\n\n",
        ]:
            with self.assertRaises(AssertionError):
                validator.validate_stream(body, item, operation, {})
        with self.assertRaises(ValidationError):
            validator.validate_stream(
                "event: next\ndata: 1\n\n" + complete, item, operation, {}
            )
        with self.assertRaises(ValidationError):
            validator.validate_stream(
                error.replace('"locked"', "1"), item, operation, {}
            )
