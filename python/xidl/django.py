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


async def request_from_django(request: Any, path_params: Optional[dict[str, str]] = None) -> Request:
    return Request(
        method=getattr(request, "method", "GET"),
        path=getattr(request, "path", "/"),
        path_params=path_params or {},
        query=normalize_query(getattr(request, "GET", {})),
        headers=normalize_headers(getattr(request, "headers", {})),
        cookies=dict(getattr(request, "COOKIES", {})),
        body=getattr(request, "body", None),
    )


async def invoke_django_route(route: Route, request: Any, **path_params: str) -> Any:
    try:
        runtime_request = await request_from_django(request, path_params or None)
        return materialize_result(await execute_route(route, runtime_request))
    except HttpError as error:
        return encode_error(error)


@dataclass
class DjangoAdapter(RouteAdapter):
    urlpatterns: list[MountedRoute] = field(default_factory=list)

    @property
    def routes(self) -> list[MountedRoute]:
        return self.urlpatterns

    def add_route(self, route: Route) -> None:
        for path in route.paths:
            normalized_path = framework_path(path, "django")

            async def endpoint(request: Any, _route: Route = route, **path_params: str) -> Any:
                return await invoke_django_route(_route, request, **path_params)

            self.urlpatterns.append(
                MountedRoute(
                    path=normalized_path,
                    methods=[route.method],
                    endpoint=endpoint,
                    route=route,
                )
            )
