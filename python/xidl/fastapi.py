from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional

from .http import (
    HttpError,
    MountedRoute,
    Request,
    Route,
    RouteAdapter,
    encode_error,
    execute_route,
    framework_path,
    materialize_result,
    normalize_headers,
    normalize_query,
)


async def request_from_fastapi(request: Any, path_params: Optional[dict[str, str]] = None) -> Request:
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


async def invoke_fastapi_route(route: Route, request: Any, **path_params: str) -> Any:
    try:
        runtime_request = await request_from_fastapi(request, path_params or None)
        return materialize_result(await execute_route(route, runtime_request))
    except HttpError as error:
        return encode_error(error)


@dataclass
class FastAPIAdapter(RouteAdapter):
    app: Any = None
    routes: list[MountedRoute] = field(default_factory=list)

    def add_route(self, route: Route) -> None:
        for path in route.paths:
            normalized_path = framework_path(path, "fastapi")

            async def endpoint(request: Any, _route: Route = route, **path_params: str) -> Any:
                return await invoke_fastapi_route(_route, request, **path_params)

            mounted = MountedRoute(
                path=normalized_path,
                methods=[route.method],
                endpoint=endpoint,
                route=route,
            )
            self.routes.append(mounted)
            if self.app is not None and hasattr(self.app, "add_api_route"):
                self.app.add_api_route(normalized_path, endpoint, methods=[route.method])
