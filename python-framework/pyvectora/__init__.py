"""
PyVectora - Rust-powered Python framework for high-performance APIs

Example usage:
    from pyvectora import App, Response

    app = App()

    @app.route("/")
    def index(request):
        return Response.json({"message": "Hello, World!"})

    app.serve()
"""

from .app import App
from .request import Request
from .response import Response
from .controller import Controller, get, post, put, delete, patch, head, options
from .di import Provider

try:
    from pyvectora.pyvectora_native import version as _native_version

    __native_available__ = True
except ImportError:
    __native_available__ = False

    def _native_version() -> str:
        return "0.1.0"

from .contract import Contract
from .guard import Guard
try:
    from .database import Database, Transaction, DatabaseError
except Exception:
    Database = None  # type: ignore
    Transaction = None  # type: ignore
    DatabaseError = None  # type: ignore
from .repository import Repository
from .response import (
    StreamingResponse,
    EventSourceResponse,
    sse_event,
    sse_json
)

PyVectora = App
DatabasePool = Database

__version__ = _native_version()
__all__ = [
    "App", "PyVectora", "Request", "Response",
    "Controller", "get", "post", "put", "delete", "patch", "head", "options",
    "Provider", "Contract", "Guard", "Database", "DatabasePool", "Transaction", "DatabaseError",
    "Repository",
    "StreamingResponse", "EventSourceResponse", "sse_event", "sse_json",
    "__native_available__", "__version__"
]
