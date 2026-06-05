from __future__ import annotations

import base64
import json
from dataclasses import asdict, dataclass, field, is_dataclass
from typing import Any, Awaitable, Callable, Iterable, Mapping, Optional, Protocol, Sequence, Union


HeaderMap = Mapping[str, Sequence[str]]
QueryMap = Mapping[str, Sequence[str]]


@dataclass
class Request:
    method: str
    path: str
    path_params: Mapping[str, str] = field(default_factory=dict)
    query: QueryMap = field(default_factory=dict)
    headers: HeaderMap = field(default_factory=dict)
    cookies: Mapping[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None


@dataclass
class Response:
    status_code: int = 200
    headers: dict[str, str] = field(default_factory=dict)
    body: Any = None


@dataclass
class HttpError(Exception):
    status_code: int
    code: str
    message: str
    headers: dict[str, str] = field(default_factory=dict)


@dataclass
class SecurityRequirement:
    kind: str
    name: Optional[str] = None
    location: Optional[str] = None
    realm: Optional[str] = None
    scopes: list[str] = field(default_factory=list)


@dataclass
class SecurityContext:
    basic_username: Optional[str] = None
    basic_password: Optional[str] = None
    bearer_token: Optional[str] = None
    api_keys: dict[str, str] = field(default_factory=dict)


@dataclass
class StreamMetadata:
    kind: str
    codec: str


@dataclass
class RouteMetadata:
    request_content_type: str
    response_content_type: str
    security: list[SecurityRequirement] = field(default_factory=list)
    stream: Optional[StreamMetadata] = None


@dataclass
class ServerStreamResponse:
    items: Iterable[Any]
    status_code: int = 200
    headers: dict[str, str] = field(default_factory=dict)


RuntimeResult = Union[Response, ServerStreamResponse]
Endpoint = Callable[[Request], Awaitable[RuntimeResult]]
ServiceHandler = Callable[[Any], Awaitable[Any]]


@dataclass
class Route:
    method: str
    paths: list[str]
    endpoint: Endpoint
    handler: ServiceHandler
    request_model: type[Any]
    response_model: type[Any]
    metadata: RouteMetadata


@dataclass
class MountedRoute:
    path: str
    methods: list[str]
    endpoint: Callable[..., Awaitable[Any]]
    route: Route


class RouteAdapter(Protocol):
    def add_route(self, route: Route) -> None:
        ...


def register_routes(adapter: RouteAdapter, routes: Sequence[Route]) -> None:
    for route in routes:
        adapter.add_route(route)


async def execute_route(route: Route, request: Request) -> RuntimeResult:
    if request.method.upper() != route.method.upper():
        raise HttpError(405, "METHOD_NOT_ALLOWED", "method does not match route")
    return await route.endpoint(request)


def require_accept(request: Request, mime: str) -> None:
    if not mime:
        return
    accept = first_header(request.headers, "accept")
    if accept in (None, "*/*"):
        return
    accepted_types = [value.split(";", 1)[0].strip().lower() for value in accept.split(",")]
    if mime.lower() in accepted_types:
        return
    if mime.endswith("/json") and "application/json" in accepted_types:
        return
    raise HttpError(406, "NOT_ACCEPTABLE", f"accept {accept!r} does not include {mime!r}")


def require_content_type(request: Request, mime: str) -> None:
    if not mime:
        return
    content_type = first_header(request.headers, "content-type")
    if content_type is None:
        raise HttpError(415, "UNSUPPORTED_MEDIA_TYPE", f"missing content type {mime!r}")
    media_type = content_type.split(";", 1)[0].strip()
    if media_type.lower() != mime.lower():
        raise HttpError(
            415,
            "UNSUPPORTED_MEDIA_TYPE",
            f"content type {media_type!r} does not match {mime!r}",
        )


def require_security(
    request: Request,
    requirements: Sequence[SecurityRequirement],
) -> Optional[SecurityContext]:
    if not requirements:
        return None
    if len(requirements) == 1 and requirements[0].kind == "none":
        return None

    auth_header = first_header(request.headers, "authorization")
    basic = parse_basic_auth(auth_header)
    bearer = parse_bearer_auth(auth_header)
    api_keys: dict[str, str] = {}

    for requirement in requirements:
        if requirement.kind == "basic" and basic is not None:
            return SecurityContext(
                basic_username=basic[0],
                basic_password=basic[1],
            )
        if requirement.kind in {"bearer", "oauth2"} and bearer is not None:
            return SecurityContext(bearer_token=bearer)
        if requirement.kind == "api_key" and requirement.name and requirement.location:
            value = find_api_key(request, requirement.location, requirement.name)
            if value is not None:
                api_keys[requirement.name] = value
                return SecurityContext(api_keys=api_keys)

    headers: dict[str, str] = {}
    for requirement in requirements:
        if requirement.kind == "basic":
            realm = requirement.realm or "xidl"
            headers["WWW-Authenticate"] = f'Basic realm="{realm}"'
            break
        if requirement.kind in {"bearer", "oauth2"}:
            headers["WWW-Authenticate"] = "Bearer"
            break
    raise HttpError(401, "UNAUTHORIZED", "missing required authentication", headers=headers)


def path_value(request: Request, name: str) -> str:
    value = request.path_params.get(name)
    if value is None or value == "":
        raise HttpError(400, "INVALID_ARGUMENT", f"missing path value {name!r}")
    return value


def query_value(request: Request, name: str) -> Optional[str]:
    return first_value(request.query, name)


def header_value(request: Request, name: str) -> Optional[str]:
    return first_header(request.headers, name)


def cookie_value(request: Request, name: str) -> Optional[str]:
    return request.cookies.get(name)


def read_scalar(
    value: Optional[str],
    ty: str,
    *,
    optional: bool,
    default_on_missing: bool,
    wire_name: str,
) -> Any:
    if value is None or value == "":
        if optional:
            return None
        if default_on_missing:
            return default_value(ty)
        raise HttpError(400, "INVALID_ARGUMENT", f"missing value for {wire_name!r}")
    try:
        return coerce_scalar(value, ty)
    except ValueError as exc:
        raise HttpError(400, "INVALID_ARGUMENT", str(exc)) from exc


def read_json_body(request: Request) -> Any:
    body = request.body or b""
    if not body:
        return {}
    try:
        return json.loads(body.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise HttpError(400, "INVALID_ARGUMENT", "invalid JSON body") from exc


def read_form_body(request: Request) -> Any:
    body = request.body or b""
    if not body:
        return {}
    try:
        from urllib.parse import parse_qs

        parsed = parse_qs(body.decode("utf-8"))
        return {k: v[0] for k, v in parsed.items()}
    except Exception as exc:
        raise HttpError(400, "INVALID_ARGUMENT", "invalid form body") from exc


def read_json_field(
    body: Any,
    name: str,
    ty: str,
    *,
    optional: bool,
) -> Any:
    if not isinstance(body, dict):
        raise HttpError(400, "INVALID_ARGUMENT", "expected JSON object body")
    if name not in body:
        if optional:
            return None
        return default_value(ty)
    return coerce_json(body[name], ty, optional=optional, wire_name=name)


def read_json_value(body: Any, ty: str, *, optional: bool, wire_name: str) -> Any:
    if body is None:
        if optional:
            return None
        return default_value(ty)
    return coerce_json(body, ty, optional=optional, wire_name=wire_name)


def encode_json_response(value: Any, status_code: int = 200) -> Response:
    if isinstance(value, Response):
        return ensure_response_header(value, "Content-Type", "application/json")
    if isinstance(value, ServerStreamResponse):
        raise HttpError(500, "INTERNAL", "stream response used in unary route")
    if value is None:
        return Response(status_code=204)
    return Response(
        status_code=status_code,
        headers={"Content-Type": "application/json"},
        body=to_body(value),
    )


def encode_stream_response(
    value: Any,
    *,
    codec: str,
    status_code: int = 200,
) -> ServerStreamResponse:
    if isinstance(value, ServerStreamResponse):
        headers = dict(value.headers)
        headers.setdefault("Content-Type", stream_content_type(codec))
        return ServerStreamResponse(items=value.items, status_code=value.status_code, headers=headers)
    raise HttpError(500, "INTERNAL", "expected ServerStreamResponse for stream route")


def framework_path(path: str, framework: str) -> str:
    trimmed = strip_query_template(path)
    if framework == "fastapi":
        return trimmed.replace("{*", "{")
    if framework == "django":
        return trimmed.replace("{*", "<path:").replace("{", "<str:").replace("}", ">")
    return trimmed


def strip_query_template(path: str) -> str:
    pos = path.find("{?")
    if pos >= 0:
        return path[:pos]
    return path


def match_path(path_template: str, path: str) -> Optional[dict[str, str]]:
    template_parts = [part for part in strip_query_template(path_template).split("/") if part]
    path_parts = [part for part in path.split("/") if part]
    params: dict[str, str] = {}

    index = 0
    while index < len(template_parts):
        template_part = template_parts[index]
        if template_part.startswith("{*") and template_part.endswith("}"):
            params[template_part[2:-1]] = "/".join(path_parts[index:])
            return params
        if index >= len(path_parts):
            return None
        path_part = path_parts[index]
        if template_part.startswith("{") and template_part.endswith("}"):
            params[template_part[1:-1]] = path_part
        elif template_part != path_part:
            return None
        index += 1

    if index != len(path_parts):
        return None
    return params


def normalize_headers(value: Any) -> dict[str, list[str]]:
    if value is None:
        return {}
    if hasattr(value, "items"):
        items = value.items()
    else:
        items = value
    out: dict[str, list[str]] = {}
    for key, raw in items:
        normalized = str(key).lower()
        if isinstance(raw, (list, tuple)):
            out[normalized] = [str(item) for item in raw]
        else:
            out[normalized] = [str(raw)]
    return out


def normalize_query(value: Any) -> dict[str, list[str]]:
    if value is None:
        return {}
    if hasattr(value, "lists"):
        return {str(key): [str(item) for item in items] for key, items in value.lists()}
    if hasattr(value, "multi_items"):
        out: dict[str, list[str]] = {}
        for key, item in value.multi_items():
            out.setdefault(str(key), []).append(str(item))
        return out
    if hasattr(value, "items"):
        out = {}
        for key, raw in value.items():
            if isinstance(raw, (list, tuple)):
                out[str(key)] = [str(item) for item in raw]
            else:
                out[str(key)] = [str(raw)]
        return out
    return {}


def encode_error(error: HttpError) -> Response:
    payload = {"code": error.status_code, "msg": error.message}
    headers = dict(error.headers)
    headers.setdefault("Content-Type", "application/json")
    return Response(status_code=error.status_code, headers=headers, body=payload)


def materialize_result(result: RuntimeResult) -> Response:
    if isinstance(result, Response):
        return result
    body = b"".join(compile_stream(result))
    headers = dict(result.headers)
    headers.setdefault("Content-Type", "application/octet-stream")
    return Response(status_code=result.status_code, headers=headers, body=body)


def compile_stream(result: ServerStreamResponse) -> Iterable[bytes]:
    codec = result.headers.get("Content-Type", "")
    if codec.startswith("text/event-stream"):
        return compile_sse_stream(result.items)
    return compile_ndjson_stream(result.items)


def compile_sse_stream(items: Iterable[Any]) -> Iterable[bytes]:
    for item in items:
        payload = json.dumps(to_body(item))
        yield f"event: next\ndata: {payload}\n\n".encode("utf-8")
    yield b"event: complete\n\n"


def compile_ndjson_stream(items: Iterable[Any]) -> Iterable[bytes]:
    for item in items:
        payload = json.dumps(to_body(item))
        yield (payload + "\n").encode("utf-8")


def stream_content_type(codec: str) -> str:
    if codec == "sse":
        return "text/event-stream"
    if codec == "ndjson":
        return "application/x-ndjson"
    return "application/octet-stream"


def ensure_response_header(response: Response, name: str, value: str) -> Response:
    headers = dict(response.headers)
    headers.setdefault(name, value)
    return Response(status_code=response.status_code, headers=headers, body=response.body)


def to_body(value: Any) -> Any:
    if is_dataclass(value):
        return _normalize_body(asdict(value))
    return _normalize_body(value)


def _normalize_body(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            ("return" if key == "return_" else key): _normalize_body(item)
            for key, item in value.items()
        }
    if isinstance(value, list):
        return [_normalize_body(item) for item in value]
    if isinstance(value, tuple):
        return [_normalize_body(item) for item in value]
    return value


def first_value(values: QueryMap, name: str) -> Optional[str]:
    items = values.get(name)
    if not items:
        return None
    return items[0]


def first_header(headers: HeaderMap, name: str) -> Optional[str]:
    values = headers.get(name.lower()) or headers.get(name) or headers.get(name.title())
    if not values:
        return None
    return values[0]


def parse_basic_auth(header: Optional[str]) -> Optional[tuple[str, str]]:
    if not header or not header.startswith("Basic "):
        return None
    try:
        decoded = base64.b64decode(header[6:]).decode("utf-8")
    except Exception:
        return None
    if ":" not in decoded:
        return None
    username, password = decoded.split(":", 1)
    return username, password


def parse_bearer_auth(header: Optional[str]) -> Optional[str]:
    if not header or not header.startswith("Bearer "):
        return None
    return header[7:]


def find_api_key(request: Request, location: str, name: str) -> Optional[str]:
    if location == "header":
        return header_value(request, name)
    if location == "query":
        return query_value(request, name)
    if location == "cookie":
        return cookie_value(request, name)
    return None


def default_value(ty: str) -> Any:
    if ty.startswith("Optional["):
        return None
    if ty == "int":
        return 0
    if ty == "float":
        return 0.0
    if ty == "bool":
        return False
    if ty == "str":
        return ""
    if ty.startswith("list["):
        return []
    if ty.startswith("dict["):
        return {}
    return None


def coerce_scalar(value: str, ty: str) -> Any:
    if ty == "str":
        return value
    if ty == "int":
        return int(value)
    if ty == "float":
        return float(value)
    if ty == "bool":
        lowered = value.lower()
        if lowered in {"true", "1"}:
            return True
        if lowered in {"false", "0"}:
            return False
        raise ValueError(f"invalid bool {value!r}")
    return value


def coerce_json(value: Any, ty: str, *, optional: bool, wire_name: str) -> Any:
    if value is None:
        if optional:
            return None
        raise HttpError(400, "INVALID_ARGUMENT", f"null is not allowed for {wire_name!r}")
    if ty == "str":
        return str(value)
    if ty == "int":
        return int(value)
    if ty == "float":
        return float(value)
    if ty == "bool":
        return bool(value)
    if ty.startswith("list["):
        if not isinstance(value, list):
            raise HttpError(400, "INVALID_ARGUMENT", f"expected list for {wire_name!r}")
        inner = ty[5:-1]
        return [coerce_json(item, inner, optional=False, wire_name=wire_name) for item in value]
    if ty.startswith("dict["):
        if not isinstance(value, dict):
            raise HttpError(400, "INVALID_ARGUMENT", f"expected object for {wire_name!r}")
        return value
    return value
