from __future__ import annotations
from typing import Any, Awaitable, Union

from .guard import Guard
from .di import Provider, register_global_provider
from .request import Request

class AuthGuard(Guard):
    """
    Guard that signals to the Rust core that this route requires authentication.

    If used, the Rust server will validate the JWT before the request even
    reaches Python. If validation fails, Rust returns 401.

    This guard also ensures Python logic knows it's protected.
    """
    def can_activate(self, context: Any) -> Union[bool, Awaitable[bool]]:
        return True

def Protected():
    """
    Decorator to mark a route as protected.

    Usage:
        @get("/private")
        @Protected()
        def handler(): ...
    """
    def decorator(handler):
        if not hasattr(handler, "__guards__"):
            handler.__guards__ = []
        handler.__guards__.append(AuthGuard())
        return handler
    return decorator

class CurrentUser(Provider):
    """
    Provider that injects the authenticated user's claims.

    Rust parses the JWT and places claims in Request.
    This provider extracts them.
    """
    async def provide(self, request: Request) -> Any:
        if hasattr(request, "claims"):
             return request.claims
        return None

register_global_provider(CurrentUser, CurrentUser())
