__test__ = False

from typing import Optional, Dict, Any, Union
import json
from .response import Response

class TestClient:
    """
    Zero-network test client for PyVectora applications.
    Executes requests directly against the Rust core, bypassing the OS network stack.
    """
    def __init__(self, app):
        """
        Initialize TestClient.

        Args:
            app: Check if it's a PyApp from bindings or our wrapper App.
        """
        if hasattr(app, "native_app"):
            self.server = app.native_app.test_client()
        else:
            self.server = app.test_client() # If native app passed directly

    def request(
        self,
        method: str,
        path: str,
        headers: Optional[Dict[str, str]] = None,
        json: Optional[Dict[str, Any]] = None,
        data: Optional[str] = None
    ) -> Response:

        body_bytes = None
        headers = headers or {}

        if json is not None:
            import json as json_lib
            body_str = json_lib.dumps(json)
            body_bytes = body_str.encode("utf-8")
            headers["Content-Type"] = "application/json"
        elif data is not None:
            body_bytes = data.encode("utf-8")
            if "Content-Type" not in headers:
                 headers["Content-Type"] = "text/plain"

        resp = self.server.test_request(
            method,
            path,
            headers,
            body_bytes
        )

        py_resp = Response(
            body=resp.body,
            status=resp.status,
            content_type=resp.content_type
        )
        if hasattr(resp, "headers"):
            py_resp.headers = resp.headers
        return py_resp

    def get(self, path: str, **kwargs) -> Response:
        return self.request("GET", path, **kwargs)

    def post(self, path: str, **kwargs) -> Response:
        return self.request("POST", path, **kwargs)

    def put(self, path: str, **kwargs) -> Response:
        return self.request("PUT", path, **kwargs)

    def delete(self, path: str, **kwargs) -> Response:
        return self.request("DELETE", path, **kwargs)
