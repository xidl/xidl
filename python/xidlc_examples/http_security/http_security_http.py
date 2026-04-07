from __future__ import annotations

import abc
from dataclasses import dataclass
from typing import Optional

from xidl.http import (
    Request,
    Route,
    RouteMetadata,
    SecurityRequirement,
    ServerStreamResponse,
    StreamMetadata,
    cookie_value,
    encode_json_response,
    encode_stream_response,
    header_value,
    path_value,
    query_value,
    read_json_body,
    read_json_field,
    read_scalar,
    require_accept,
    require_content_type,
    require_security,
)

# generated http module: http_security

@dataclass
class HttpSecurityServiceGetUserRequest:
    id: int
    locale: Optional[str]
    trace_id: str

@dataclass
class HttpSecurityServiceGetUserResponse:
    value: str

@dataclass
class HttpSecurityServiceSearchUserRequest:
    keyword: str
    page: Optional[int]

@dataclass
class HttpSecurityServiceSearchUserResponse:
    value: str

@dataclass
class HttpSecurityServiceHealthRequest:
    pass

@dataclass
class HttpSecurityServiceHealthResponse:
    value: str

class HttpSecurityServiceService(abc.ABC):
    @abc.abstractmethod
    async def get_user(self, request: HttpSecurityServiceGetUserRequest) -> HttpSecurityServiceGetUserResponse:
        raise NotImplementedError

    @abc.abstractmethod
    async def search_user(self, request: HttpSecurityServiceSearchUserRequest) -> HttpSecurityServiceSearchUserResponse:
        raise NotImplementedError

    @abc.abstractmethod
    async def health(self, request: HttpSecurityServiceHealthRequest) -> HttpSecurityServiceHealthResponse:
        raise NotImplementedError

async def _http_security_service_get_user_endpoint(service: HttpSecurityServiceService, request: Request):
    require_accept(request, "application/json")
    _security = require_security(request, [SecurityRequirement(kind="basic"), SecurityRequirement(kind="api_key", name="X-API-Key", location="header")])
    request_value = HttpSecurityServiceGetUserRequest(
        id=read_scalar(path_value(request, "id"), "int", optional=False, default_on_missing=False, wire_name="id"),
        locale=read_scalar(query_value(request, "locale"), "str", optional=True, default_on_missing=False, wire_name="locale"),
        trace_id=read_scalar(header_value(request, "X-Trace-Id"), "str", optional=False, default_on_missing=False, wire_name="X-Trace-Id"),
    )
    response_value = await service.get_user(request_value)
    return encode_json_response(response_value)

def _http_security_service_get_user_route(service: HttpSecurityServiceService) -> Route:
    async def endpoint(request: Request):
        return await _http_security_service_get_user_endpoint(service, request)
    return Route(
        method="GET",
        paths=["/users/{id}"],
        endpoint=endpoint,
        handler=service.get_user,
        request_model=HttpSecurityServiceGetUserRequest,
        response_model=HttpSecurityServiceGetUserResponse,
        metadata=RouteMetadata(request_content_type="application/json", response_content_type="application/json", security=[SecurityRequirement(kind="basic"), SecurityRequirement(kind="api_key", name="X-API-Key", location="header")], stream=None),
    )

async def _http_security_service_search_user_endpoint(service: HttpSecurityServiceService, request: Request):
    require_accept(request, "application/json")
    require_content_type(request, "application/json")
    _security = require_security(request, [SecurityRequirement(kind="oauth2", scopes=["write:users", "read:users"])])
    body = read_json_body(request)
    request_value = HttpSecurityServiceSearchUserRequest(
        keyword=read_json_field(body, "keyword", "str", optional=False),
        page=read_json_field(body, "page", "int", optional=True),
    )
    response_value = await service.search_user(request_value)
    return encode_json_response(response_value)

def _http_security_service_search_user_route(service: HttpSecurityServiceService) -> Route:
    async def endpoint(request: Request):
        return await _http_security_service_search_user_endpoint(service, request)
    return Route(
        method="POST",
        paths=["/users/search"],
        endpoint=endpoint,
        handler=service.search_user,
        request_model=HttpSecurityServiceSearchUserRequest,
        response_model=HttpSecurityServiceSearchUserResponse,
        metadata=RouteMetadata(request_content_type="application/json", response_content_type="application/json", security=[SecurityRequirement(kind="oauth2", scopes=["write:users", "read:users"])], stream=None),
    )

async def _http_security_service_health_endpoint(service: HttpSecurityServiceService, request: Request):
    require_accept(request, "application/json")
    _security = require_security(request, [SecurityRequirement(kind="none")])
    request_value = HttpSecurityServiceHealthRequest()
    response_value = await service.health(request_value)
    return encode_json_response(response_value)

def _http_security_service_health_route(service: HttpSecurityServiceService) -> Route:
    async def endpoint(request: Request):
        return await _http_security_service_health_endpoint(service, request)
    return Route(
        method="POST",
        paths=["/health"],
        endpoint=endpoint,
        handler=service.health,
        request_model=HttpSecurityServiceHealthRequest,
        response_model=HttpSecurityServiceHealthResponse,
        metadata=RouteMetadata(request_content_type="application/json", response_content_type="application/json", security=[SecurityRequirement(kind="none")], stream=None),
    )

def http_security_service_routes(service: HttpSecurityServiceService) -> list[Route]:
    return [
        _http_security_service_get_user_route(service),
        _http_security_service_search_user_route(service),
        _http_security_service_health_route(service),
    ]

