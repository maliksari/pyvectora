from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable, List

if TYPE_CHECKING:
    from .app import App

class Route:
    """Internal route representation."""

    def __init__(self, method: str, path: str, handler: Callable[..., Any]) -> None:
        self.method = method
        self.path = path
        self.handler = handler

class Router:
    """
    Router class for grouping path operations.
    Supports hierarchical routing with prefixes and tags.
    """

    def __init__(self, prefix: str = "", tags: List[str] | None = None) -> None:
        self.prefix = prefix
        self.tags = tags or []
        self._routes: List[Route] = []

    def route(
        self, path: str, methods: list[str] | None = None
    ) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        """
        Register a route with multiple HTTP methods.
        """
        if methods is None:
            methods = ["GET"]

        def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
            route_path = path

            for method in methods:
                self._routes.append(Route(method.upper(), route_path, func))
            return func

        return decorator

    def get(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["GET"])

    def post(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["POST"])

    def put(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["PUT"])

    def delete(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["DELETE"])

    def patch(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["PATCH"])

    def head(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["HEAD"])

    def options(self, path: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
        return self.route(path, ["OPTIONS"])

    def include_router(self, router: 'Router', prefix: str = "") -> None:
        """
        Include another router's routes into this router.
        """
        for route in router._routes:

            combined_prefix = (prefix + router.prefix).rstrip("/")
            if not combined_prefix.startswith("/") and combined_prefix:
                combined_prefix = "/" + combined_prefix

            final_path = combined_prefix + route.path
            if not final_path.startswith("/"):
                final_path = "/" + final_path

            new_route = Route(route.method, final_path, route.handler)
            self._routes.append(new_route)
