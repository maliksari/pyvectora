"""
PyVectora Dependency Injection System

High-performance DI with startup-time type resolution caching.

Design Principles:
- S: Single module for DI resolution
- O: Extensible provider system
- L: All providers implement Provider interface
- D: Handlers depend on abstractions (Provider types)

Performance Optimization:
- Type hints resolved at STARTUP (register_handler)
- Per-request: Fast cache lookup only
- Eliminates costly get_type_hints() on hot path
"""

from __future__ import annotations
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, Type, TypeVar, get_type_hints, Callable, List, Optional, Tuple
import inspect

T = TypeVar("T")

class Provider(ABC):
    """
    Base class for all Dependency Providers.
    Implement the `provide` method to define how to create the dependency.
    """

    @abstractmethod
    async def provide(self, request: Any) -> Any:
        """Provide the dependency value for a given request."""
        pass

@dataclass
class ResolvedParam:
    """Pre-computed parameter resolution info."""
    name: str
    param_type: Optional[Type]
    is_request: bool = False
    is_contract: bool = False
    is_provider: bool = False
    contract_class: Optional[Type] = None

@dataclass
class HandlerMetadata:
    """
    Cached metadata for a handler function.
    Computed once at startup, reused for all requests.
    """
    handler: Callable
    params: List[ResolvedParam] = field(default_factory=list)
    is_async: bool = False
    guards: List[Type] = field(default_factory=list)

class Injector:
    """
    Central Dependency Injection Container with caching.

    Performance:
    - register_handler(): Called once at startup, caches type info
    - resolve_cached(): Called per request, uses cached info
    - resolve(): Legacy method, no caching (for backwards compat)
    """

    def __init__(self):
        self._providers: Dict[Type[Any], Provider] = {}
        self._handler_cache: Dict[int, HandlerMetadata] = {}  # id(handler) -> metadata

    def register(self, interface: Type[Any], provider: Provider):
        """Register a provider instance for a specific type/interface."""
        self._providers[interface] = provider

    def register_handler(self, handler: Callable, guards: List[Type] = None) -> HandlerMetadata:
        """
        Pre-compute and cache handler metadata at STARTUP time.

        This eliminates get_type_hints() and inspect calls from hot path.

        Returns:
            HandlerMetadata with pre-resolved parameter info
        """
        handler_id = id(handler)

        if handler_id in self._handler_cache:
            return self._handler_cache[handler_id]

        try:
            hints = get_type_hints(handler)
        except Exception:
            hints = {}

        sig = inspect.signature(handler)

        params: List[ResolvedParam] = []

        for param_name, param in sig.parameters.items():
            if param_name == 'return':
                continue

            param_type = hints.get(param_name)

            if param_type is None and param.annotation is not inspect.Parameter.empty:
                param_type = param.annotation

            resolved = ResolvedParam(name=param_name, param_type=param_type)

            type_name = getattr(param_type, "__name__", "")
            if isinstance(param_type, str):
                type_name = param_type

            if type_name in ('Request', 'PyRequest'):
                resolved.is_request = True

            elif param_type and param_type in self._providers:
                resolved.is_provider = True

            elif param_type and isinstance(param_type, type):
                try:
                    from .contract import Contract
                    if issubclass(param_type, Contract):
                        resolved.is_contract = True
                        resolved.contract_class = param_type
                except ImportError:
                    pass

            params.append(resolved)

        metadata = HandlerMetadata(
            handler=handler,
            params=params,
            is_async=inspect.iscoroutinefunction(handler),
            guards=guards or []
        )

        self._handler_cache[handler_id] = metadata

        return metadata

    async def resolve_cached(self, metadata: HandlerMetadata, request: Any) -> Dict[str, Any]:
        """
        Fast resolution using cached metadata.

        This is the HOT PATH - no reflection calls here.
        """
        injections = {}

        for param in metadata.params:
            if param.is_request:
                injections[param.name] = request

            elif param.is_provider and param.param_type in self._providers:
                provider = self._providers[param.param_type]
                injections[param.name] = await provider.provide(request)

            elif param.is_contract and param.contract_class:
                body_str = getattr(request, "body", "{}")
                if not body_str:
                    body_str = "{}"
                try:
                    injections[param.name] = param.contract_class.from_json(body_str)
                except ValueError as e:
                    raise ValueError(f"Contract Validation Error for '{param.name}': {e}") from e

        return injections

    async def resolve(self, target_func: Callable, request: Any) -> Dict[str, Any]:
        """
        Legacy resolution method (no caching).

        For backwards compatibility. New code should use:
        1. register_handler() at startup
        2. resolve_cached() per request
        """
        handler_id = id(target_func)
        if handler_id in self._handler_cache:
            return await self.resolve_cached(self._handler_cache[handler_id], request)

        try:
            hints = get_type_hints(target_func)
        except Exception:
            hints = {}

        injections = {}
        sig = inspect.signature(target_func)

        for param_name, param in sig.parameters.items():
            if param_name == 'return':
                continue

            param_type = hints.get(param_name)

            if param_type is None and param.annotation is not inspect.Parameter.empty:
                param_type = param.annotation

            if param_type and param_type in self._providers:
                provider = self._providers[param_type]
                injections[param_name] = await provider.provide(request)

            type_name = getattr(param_type, "__name__", "")
            if isinstance(param_type, str):
                type_name = param_type

            if param_type and type_name in ('Request', 'PyRequest'):
                injections[param_name] = request

            elif param_type and isinstance(param_type, type):
                try:
                    from .contract import Contract
                    if issubclass(param_type, Contract):
                        body_str = getattr(request, "body", "{}")
                        if not body_str:
                            body_str = "{}"
                        try:
                            injections[param_name] = param_type.from_json(body_str)
                        except ValueError as e:
                            raise ValueError(f"Contract Validation Error for '{param_name}': {e}") from e
                except ImportError:
                    pass

        return injections

_injector = Injector()

def get_injector() -> Injector:
    """Get the global Injector instance."""
    return _injector

def register_global_provider(interface: Type[Any], provider: Provider):
    """Register a provider globally."""
    _injector.register(interface, provider)

async def solve_dependencies(func: Callable, request: Any) -> Dict[str, Any]:
    """Resolve dependencies for a function (legacy API)."""
    return await _injector.resolve(func, request)

from .guard import Guard

async def execute_guards(guards: List[Type[Guard]], request: Any) -> None:
    """Execute a list of guards."""
    for guard_cls in guards:
        guard = guard_cls()
        result = guard.can_activate(request)

        if inspect.isawaitable(result):
            allow = await result
        else:
            allow = result

        if not allow:
            raise PermissionError(f"Guard {guard_cls.__name__} denied access")

def wrap_handler_with_di(
    handler: Callable[..., Any],
    guards: List[Type[Guard]] = None
) -> Callable[..., Any]:
    """
    Wraps a controller method to automatically inject dependencies.

    Optimization: Caches handler metadata on first call.
    """
    guards = guards or []
    _cached_metadata: Optional[HandlerMetadata] = None
    _valid_param_names: Optional[set] = None

    async def wrapper(request: Any, *args, **raw_kwargs) -> Any:
        nonlocal _cached_metadata, _valid_param_names

        if _cached_metadata is None:
            _cached_metadata = _injector.register_handler(handler, guards)
            _valid_param_names = {p.name for p in _cached_metadata.params}

        if _cached_metadata.guards:
            await execute_guards(_cached_metadata.guards, request)

        di_kwargs = await _injector.resolve_cached(_cached_metadata, request)

        path_params = {}
        if hasattr(request, "params"):
            try:
                all_params = request.params
                path_params = {
                    k: v for k, v in all_params.items()
                    if k in _valid_param_names
                }
            except Exception:
                pass

        final_kwargs = {**path_params, **raw_kwargs, **di_kwargs}

        if _cached_metadata.is_async:
            return await handler(**final_kwargs)
        else:
            return handler(**final_kwargs)

    return wrapper
