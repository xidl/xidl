import base64
import json
import unittest
from dataclasses import dataclass

from xidl.django import DjangoAdapter
from xidl.fastapi import FastAPIAdapter
from xidl.http import encode_json_response, register_routes
from tests.generated.http_security.http_security_http import (
    HttpSecurityServiceGetUserResponse,
    HttpSecurityServiceHealthResponse,
    HttpSecurityServiceSearchUserResponse,
    HttpSecurityServiceService,
    http_security_service_routes,
)
from tests.generated.http_stream.http_stream_http import (
    HttpStreamApiService,
    HttpStreamApiUploadAssetResponse,
    ServerStreamResponse,
    http_stream_api_routes,
)


class SecurityService(HttpSecurityServiceService):
    async def get_user(self, request):
        return HttpSecurityServiceGetUserResponse(value=str(request.id))

    async def search_user(self, request):
        return HttpSecurityServiceSearchUserResponse(value=request.keyword)

    async def health(self, request):
        return HttpSecurityServiceHealthResponse(value="ok")


class StreamService(HttpStreamApiService):
    async def alerts(self, request):
        return ServerStreamResponse(items=[request.district])

    async def upload_asset(self, request):
        return HttpStreamApiUploadAssetResponse(value=request.asset_id)


def basic_auth(username, password):
    token = base64.b64encode(f"{username}:{password}".encode("utf-8")).decode("ascii")
    return f"Basic {token}"


class FakeDjangoRequest:
    def __init__(self, *, method, path, query=None, headers=None, body=b"", cookies=None):
        self.method = method
        self.path = path
        self.GET = query or {}
        self.headers = headers or {}
        self.body = body
        self.COOKIES = cookies or {}


class FakeURL:
    def __init__(self, path):
        self.path = path


class FakeFastAPIRequest:
    def __init__(self, *, method, path, query=None, headers=None, body=b"", cookies=None, path_params=None):
        self.method = method
        self.url = FakeURL(path)
        self.query_params = query or {}
        self.headers = headers or {}
        self._body = body
        self.cookies = cookies or {}
        self.path_params = path_params or {}

    async def body(self):
        return self._body


class FakeFastAPIApp:
    def __init__(self):
        self.calls = []

    def add_api_route(self, path, endpoint, methods):
        self.calls.append((path, endpoint, methods))


class AdapterTests(unittest.IsolatedAsyncioTestCase):
    async def test_encode_json_response_renames_return_field(self):
        @dataclass
        class Payload:
            return_: int
            count: int

        response = encode_json_response(Payload(return_=1, count=2))
        self.assertEqual(200, response.status_code)
        self.assertEqual({"return": 1, "count": 2}, response.body)

    async def test_django_adapter_executes_security_route(self):
        adapter = DjangoAdapter()
        register_routes(adapter, http_security_service_routes(SecurityService()))
        self.assertEqual(3, len(adapter.routes))
        self.assertEqual(["GET"], adapter.routes[0].methods)
        self.assertEqual("/users/<str:id>", adapter.routes[0].path)
        self.assertEqual("basic", adapter.routes[0].route.metadata.security[0].kind)

        response = await adapter.routes[0].endpoint(
            FakeDjangoRequest(
                method="GET",
                path="/users/7",
                query={"locale": "zh-CN"},
                headers={
                    "accept": "text/plain",
                    "authorization": basic_auth("demo", "secret"),
                    "x-api-key": "k1",
                    "x-trace-id": "trace-1",
                },
            ),
            id="7",
        )
        self.assertEqual(200, response.status_code)
        self.assertEqual(b"7", response.body)
        self.assertEqual("text/plain", response.headers["Content-Type"])

    async def test_django_adapter_returns_unauthorized_response(self):
        adapter = DjangoAdapter()
        register_routes(adapter, http_security_service_routes(SecurityService()))

        response = await adapter.routes[0].endpoint(
            FakeDjangoRequest(
                method="GET",
                path="/users/7",
                headers={
                    "accept": "text/plain",
                    "x-trace-id": "trace-1",
                },
            ),
            id="7",
        )
        self.assertEqual(401, response.status_code)
        self.assertEqual(401, response.body["code"])
        self.assertEqual("application/json", response.headers["Content-Type"])
        self.assertIn("WWW-Authenticate", response.headers)

    async def test_fastapi_adapter_executes_stream_route(self):
        app = FakeFastAPIApp()
        adapter = FastAPIAdapter(app=app)
        register_routes(adapter, http_stream_api_routes(StreamService()))
        self.assertEqual(2, len(adapter.routes))
        self.assertEqual(2, len(app.calls))
        self.assertEqual("server", adapter.routes[0].route.metadata.stream.kind)
        self.assertEqual("ndjson", adapter.routes[1].route.metadata.stream.codec)

        response = await adapter.routes[0].endpoint(
            FakeFastAPIRequest(
                method="GET",
                path="/alerts/north",
                headers={
                    "accept": "text/event-stream",
                    "authorization": basic_auth("demo", "secret"),
                },
                path_params={"district": "north"},
            )
        )
        self.assertEqual(200, response.status_code)
        self.assertEqual("text/event-stream", response.headers["Content-Type"])
        self.assertIn(b"event: next", response.body)
        self.assertIn(b"north", response.body)

    async def test_fastapi_registered_unary_endpoint_executes(self):
        app = FakeFastAPIApp()
        adapter = FastAPIAdapter(app=app)
        register_routes(adapter, http_security_service_routes(SecurityService()))

        path, endpoint, methods = app.calls[1]
        self.assertEqual("/users/search", path)
        self.assertEqual(["POST"], methods)

        response = await endpoint(
            FakeFastAPIRequest(
                method="POST",
                path="/users/search",
                headers={
                    "accept": "text/plain",
                    "authorization": "Bearer token-1",
                    "content-type": "application/json",
                },
                body=json.dumps({"keyword": "alice", "page": 2}).encode("utf-8"),
            )
        )
        self.assertEqual(200, response.status_code)
        self.assertEqual(b"alice", response.body)
        self.assertEqual("text/plain", response.headers["Content-Type"])

    async def test_fastapi_registered_unary_endpoint_rejects_invalid_content_type(self):
        app = FakeFastAPIApp()
        adapter = FastAPIAdapter(app=app)
        register_routes(adapter, http_security_service_routes(SecurityService()))

        _, endpoint, _ = app.calls[1]
        response = await endpoint(
            FakeFastAPIRequest(
                method="POST",
                path="/users/search",
                headers={
                    "accept": "text/plain",
                    "authorization": "Bearer token-1",
                    "content-type": "text/plain",
                },
                body=b"keyword=alice",
            )
        )
        self.assertEqual(415, response.status_code)
        body = json.loads(response.body)
        self.assertEqual(415, body["code"])
        self.assertEqual("application/json", response.headers["Content-Type"])


if __name__ == "__main__":
    unittest.main()
