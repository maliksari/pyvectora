"""
PyVectora Response - Response objects for handlers.

Provides helper classes for creating HTTP responses.
"""

from __future__ import annotations

import json
from typing import Any

class Response:
    """
    HTTP Response object.

    Attributes:
        status: HTTP status code (default: 200)
        body: Response body as string
        content_type: Content-Type header value
    """

    def __init__(
        self,
        body: str = "",
        status: int = 200,
        content_type: str = "application/json",
    ) -> None:
        """Initialize a Response object."""
        self.status = status
        self.body = body
        self.content_type = content_type
        self.headers: dict[str, str] = {}

    @classmethod
    def json(cls, data: dict[str, Any] | list[Any], status: int = 200) -> Response:
        """
        Create a JSON response.

        Args:
            data: Data to serialize as JSON
            status: HTTP status code (default: 200)

        Returns:
            Response object with JSON content
        """
        return cls(
            body=json.dumps(data, ensure_ascii=False),
            status=status,
            content_type="application/json",
        )

    @classmethod
    def text(cls, text: str, status: int = 200) -> Response:
        """
        Create a plain text response.

        Args:
            text: Text content
            status: HTTP status code (default: 200)

        Returns:
            Response object with text content
        """
        return cls(
            body=text,
            status=status,
            content_type="text/plain",
        )

    @classmethod
    def html(cls, html: str, status: int = 200) -> Response:
        """
        Create an HTML response.

        Args:
            html: HTML content
            status: HTTP status code (default: 200)

        Returns:
            Response object with HTML content
        """
        return cls(
            body=html,
            status=status,
            content_type="text/html",
        )

    def with_status(self, status: int) -> Response:
        """Set the status code (Builder pattern)."""
        self.status = status
        return self

    def with_header(self, key: str, value: str) -> Response:
        """Set a header (Builder pattern). Currently supports Content-Type."""
        if key.lower() == "content-type":
            self.content_type = value
        else:
            self.headers[key] = value
        return self

    def __repr__(self) -> str:
        return f"Response(status={self.status}, content_type={self.content_type!r})"

class JSONResponse(Response):
    """Convenience class for JSON responses."""

    def __init__(self, data: dict[str, Any] | list[Any], status: int = 200) -> None:
        super().__init__(
            body=json.dumps(data, ensure_ascii=False),
            status=status,
            content_type="application/json",
        )

class TextResponse(Response):
    """Convenience class for text responses."""

    def __init__(self, text: str, status: int = 200) -> None:
        super().__init__(
            body=text,
            status=status,
            content_type="text/plain",
        )

from typing import AsyncIterator, Iterator, Union, Callable
import asyncio

class StreamingResponse:
    """
    Streaming HTTP response for large content or real-time data.

    Supports both sync and async generators.

    Example:
        async def generate_data():
            for i in range(10):
                yield f"chunk-{i}\\n"
                await asyncio.sleep(0.1)

        return StreamingResponse(generate_data())
    """

    def __init__(
        self,
        content: Union[Iterator[str], AsyncIterator[str], Callable[[], Union[Iterator[str], AsyncIterator[str]]]],
        status: int = 200,
        content_type: str = "text/plain",
        headers: dict[str, str] | None = None
    ):
        """
        Initialize a streaming response.

        Args:
            content: Sync or async generator/iterator yielding string chunks
            status: HTTP status code
            content_type: Content-Type header
            headers: Additional headers
        """
        self.content = content
        self.status = status
        self.content_type = content_type
        self.headers = headers or {}
        self._is_streaming = True

    async def collect(self) -> str:
        """Collect all chunks into a single string (for testing)."""
        chunks = []

        content = self.content() if callable(self.content) else self.content

        if hasattr(content, '__anext__'):
            async for chunk in content:
                chunks.append(chunk)
        else:
            for chunk in content:
                chunks.append(chunk)

        return "".join(chunks)

    def __repr__(self) -> str:
        return f"StreamingResponse(status={self.status}, content_type={self.content_type!r})"

class EventSourceResponse(StreamingResponse):
    """
    Server-Sent Events (SSE) response for real-time streaming.

    Perfect for:
    - LLM token streaming
    - Real-time updates
    - Live notifications

    SSE Format:
        data: <message>\\n\\n
        event: <event-type>\\ndata: <message>\\n\\n

    Example:
        async def stream_tokens():
            for token in ["Hello", " ", "World", "!"]:
                yield token
                await asyncio.sleep(0.1)

        return EventSourceResponse(stream_tokens())

    Client-side usage:
        const eventSource = new EventSource('/stream');
        eventSource.onmessage = (e) => console.log(e.data);
    """

    def __init__(
        self,
        content: Union[Iterator[str], AsyncIterator[str], Callable[[], Union[Iterator[str], AsyncIterator[str]]]],
        status: int = 200,
        event_type: str | None = None,
        headers: dict[str, str] | None = None
    ):
        """
        Initialize an SSE response.

        Args:
            content: Generator yielding event data
            status: HTTP status code
            event_type: Optional event type (SSE 'event:' field)
            headers: Additional headers
        """
        super().__init__(
            content=content,
            status=status,
            content_type="text/event-stream",
            headers=headers
        )
        self.event_type = event_type

        self.headers.update({
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",  # Disable nginx buffering
        })

    def format_event(self, data: str, event: str | None = None, id: str | None = None) -> str:
        """
        Format a single SSE event.

        Args:
            data: Event data
            event: Optional event type
            id: Optional event ID

        Returns:
            SSE-formatted string
        """
        lines = []

        if id:
            lines.append(f"id: {id}")

        if event or self.event_type:
            lines.append(f"event: {event or self.event_type}")

        for line in data.split("\n"):
            lines.append(f"data: {line}")

        lines.append("")  # Empty line to end event
        lines.append("")

        return "\n".join(lines)

    async def collect(self) -> str:
        """Collect all events into a single SSE-formatted string (for testing)."""
        chunks = []

        content = self.content() if callable(self.content) else self.content

        if hasattr(content, '__anext__'):
            async for chunk in content:
                chunks.append(self.format_event(chunk))
        else:
            for chunk in content:
                chunks.append(self.format_event(chunk))

        return "".join(chunks)

    def __repr__(self) -> str:
        return f"EventSourceResponse(status={self.status}, event_type={self.event_type!r})"

def sse_event(data: str, event: str | None = None, id: str | None = None) -> str:
    """
    Format a single SSE event string.

    Args:
        data: Event data
        event: Optional event type
        id: Optional event ID

    Returns:
        SSE-formatted string

    Example:
        yield sse_event("Hello World")

        yield sse_event("token", event="token")
    """
    lines = []

    if id:
        lines.append(f"id: {id}")

    if event:
        lines.append(f"event: {event}")

    for line in data.split("\n"):
        lines.append(f"data: {line}")

    lines.append("")
    lines.append("")

    return "\n".join(lines)

def sse_json(data: dict | list, event: str | None = None, id: str | None = None) -> str:
    """
    Format a JSON object as an SSE event.

    Args:
        data: JSON-serializable data
        event: Optional event type
        id: Optional event ID

    Returns:
        SSE-formatted string with JSON data
    """
    return sse_event(json.dumps(data, ensure_ascii=False), event=event, id=id)
