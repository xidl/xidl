from __future__ import annotations

import inspect
from dataclasses import dataclass, field
from typing import Any, Optional

import fastapi
from fastapi.responses import JSONResponse, Response as RawResponse

from .http import (
    HttpError,
    MountedRoute,
    Request,
    Response,
    Route,
    RouteAdapter,
    encode_error,
    execute_route,
    framework_path,
    materialize_result,
    normalize_headers,
    normalize_query,
)


async def request_from_fastapi(request: fastapi.Request, path_params: Optional[dict[str, str]] = None) -> Request:
    url = getattr(request, "url", None)
    body = await request.body() if hasattr(request, "body") else None
    return Request(
        method=getattr(request, "method", "GET"),
        path=getattr(url, "path", getattr(request, "path", "/")),
        path_params=path_params or dict(getattr(request, "path_params", {})),
        query=normalize_query(getattr(request, "query_params", {})),
        headers=normalize_headers(getattr(request, "headers", {})),
        cookies=dict(getattr(request, "cookies", {})),
        body=body,
    )


async def invoke_fastapi_route(route: Route, request: fastapi.Request) -> Any:
    try:
        # FastAPI passes path parameters as kwargs to the endpoint,
        # but we can also extract them directly from the request object.
        runtime_request = await request_from_fastapi(request)
        result = await execute_route(route, runtime_request)
        if isinstance(result, Response):
            if isinstance(result.body, (dict, list)):
                return JSONResponse(
                    content=result.body,
                    status_code=result.status_code,
                    headers=result.headers,
                )
            return RawResponse(
                content=result.body,
                status_code=result.status_code,
                headers=result.headers,
            )
        # Handle ServerStreamResponse
        response = materialize_result(result)
        return RawResponse(
            content=response.body,
            status_code=response.status_code,
            headers=response.headers,
        )
    except HttpError as error:
        err_resp = encode_error(error)
        return JSONResponse(
            content=err_resp.body,
            status_code=err_resp.status_code,
            headers=err_resp.headers,
        )


@dataclass
class FastAPIAdapter(RouteAdapter):
    app: Any = None
    routes: list[MountedRoute] = field(default_factory=list)

    def add_route(self, route: Route) -> None:
        for path in route.paths:
            normalized_path = framework_path(path, "fastapi")

            # Capture route in a closure. We only need the request object
            # as it contains all necessary information including path parameters.
            # We use **kwargs to catch path parameters injected by FastAPI,
            # but we override the signature so FastAPI doesn't try to validate them.
            def create_endpoint(r: Route):
                async def endpoint(request: fastapi.Request, **kwargs) -> Any:
                    return await invoke_fastapi_route(r, request)

                # Override the signature to hide **kwargs from FastAPI's validation.
                # FastAPI will still pass path parameters as keyword arguments,
                # and Python will accept them because of **kwargs.
                endpoint.__signature__ = inspect.Signature([
                    inspect.Parameter("request", inspect.Parameter.POSITIONAL_OR_KEYWORD, annotation=fastapi.Request)
                ])
                return endpoint

            endpoint = create_endpoint(route)

            mounted = MountedRoute(
                path=normalized_path,
                methods=[route.method],
                endpoint=endpoint,
                route=route,
            )
            self.routes.append(mounted)
            if self.app is not None and hasattr(self.app, "add_api_route"):
                self.app.add_api_route(normalized_path, endpoint, methods=[route.method])
