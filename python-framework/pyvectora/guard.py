from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Any, Union, Awaitable

class Guard(ABC):
    """
    Base class for all Guards.
    Guards are responsible for determining whether a request should be handled
    by the route handler or not. They are typically used for permissions,
    authentication, and throttling.

    This concept is distinct from Dependency Injection (Providers).
    """

    @abstractmethod
    def can_activate(self, context: Any) -> Union[bool, Awaitable[bool]]:
        """
        Return `True` to allow the request to proceed.
        Return `False` to deny access (raises 403 Forbidden by default).
        Raise an exception to customize the error.

        Args:
            context: The Request object (or ExecutionContext in future).
        """
        pass
