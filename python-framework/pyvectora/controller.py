from __future__ import annotations
from typing import Callable, List, Type, Any, Optional
from dataclasses import dataclass, field

@dataclass
class RouteMeta:
    """Metadata for a single route definition."""
    method: str
    path: str
    handler_name: str
    guards: List[Type[Any]] = field(default_factory=list) # Guards specific to this route

@dataclass
class ControllerMeta:
    """Metadata container for a controller class."""
    prefix: str
    tags: List[str] = field(default_factory=list)
    guards: List[Type[Any]] = field(default_factory=list) # Guards applied to all routes in class
    routes: List[RouteMeta] = field(default_factory=list)

def Controller(prefix: str = "", tags: Optional[List[str]] = None, guards: Optional[List[Type[Any]]] = None):
    """
    Class decorator for defining a Controller.

    Usage:
        @Controller("/users", guards=[AuthGuard])
        class UserController:
            ...
    """
    def decorator(cls: Type):
        meta = ControllerMeta(prefix=prefix, tags=tags or [], guards=guards or [])

        for name, method in cls.__dict__.items():
            if hasattr(method, "_route_meta"):
                route_data: RouteMeta = method._route_meta
                route_data.handler_name = name
                meta.routes.append(route_data)

        setattr(cls, "_controller_meta", meta)
        return cls
    return decorator

def _route_decorator(method: str, path: str, guards: Optional[List[Type[Any]]] = None):
    """Factory for HTTP method decorators."""
    def decorator(func: Callable):
        func._route_meta = RouteMeta(
            method=method.upper(),
            path=path,
            handler_name=func.__name__,
            guards=guards or []
        )
        return func
    return decorator

def get(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("GET", path, guards)

def post(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("POST", path, guards)

def put(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("PUT", path, guards)

def delete(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("DELETE", path, guards)

def patch(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("PATCH", path, guards)

def head(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("HEAD", path, guards)

def options(path: str = "/", guards: Optional[List[Type[Any]]] = None):
    return _route_decorator("OPTIONS", path, guards)
