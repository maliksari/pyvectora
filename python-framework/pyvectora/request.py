"""
PyVectora Request - Request object passed to handlers.

This module provides a pure-Python fallback for the Request object.
The actual Request comes from the Rust runtime during execution.
"""

from __future__ import annotations

from typing import Any

class Request:
    """
    HTTP Request object.

    Attributes:
        method: HTTP method (GET, POST, etc.)
        path: Request path
        params: Path parameters extracted from the route
        body: Raw request body as string

    Note:
        During actual execution, this is replaced by the Rust-backed Request object.
    """

    def __init__(
        self,
        method: str = "GET",
        path: str = "/",
        params: dict[str, str] | None = None,
        body: str | None = None,
        claims: dict[str, Any] | None = None,
    ) -> None:
        """Initialize a Request object (for testing/development)."""
        self._method = method
        self._path = path
        self._params = params or {}
        self._body = body
        self._claims = claims

    @property
    def method(self) -> str:
        """HTTP method (GET, POST, etc.)."""
        return self._method

    @property
    def path(self) -> str:
        """Request path."""
        return self._path

    @property
    def params(self) -> dict[str, str]:
        """Path parameters extracted from the route pattern."""
        return self._params

    @property
    def body(self) -> str | None:
        """Raw request body as string."""
        return self._body

    @property
    def text(self) -> str | None:
        """Request body as text (alias of body)."""
        return self._body

    @property
    def claims(self) -> dict[str, Any] | None:
        """Validated JWT claims (if authenticated)."""
        return self._claims

    def json(self) -> dict[str, Any]:
        """
        Parse request body as JSON.

        Returns:
            Parsed JSON as dictionary

        Raises:
            ValueError: If body is not valid JSON
        """
        import json

        if not self._body:
            return {}
        return json.loads(self._body)

    def __repr__(self) -> str:
        return f"Request(method={self.method!r}, path={self.path!r})"
