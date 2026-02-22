"""
PyVectora App - Main application class for the framework.

The App class is the primary interface for building APIs with PyVectora.
It provides decorator-based routing and delegates to the Rust runtime for execution.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable, Type, List
import asyncio
from dataclasses import dataclass

from .di import Provider, register_global_provider, wrap_handler_with_di
from .controller import ControllerMeta
from .auth import AuthGuard

if TYPE_CHECKING:
    pass

@dataclass
class Route:
    """Internal route representation."""
    method: str
    path: str
    handler: Callable[..., Any]
    auth: bool = False

class App:
    """
    PyVectora Enterprise Application.

    Object-Oriented, Controller-based structure.
    """

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 8000,
        lifespan: Callable[[App], Any] | None = None,
        enable_health_check: bool = True
    ) -> None:
        """
        Initialize a new PyVectora application.

        Args:
            host: Server host address
            port: Server port
            lifespan: Optional async context manager for startup/shutdown
            enable_health_check: Enable /health endpoint (default: True)
        """
        self.host = host
        self.port = port
        self.lifespan = lifespan
        self.enable_health_check = enable_health_check

        self._routes: List[Route] = []
        self._controllers: List[Any] = []
        self._jwt_secret: str | None = None
        self._middlewares: List[tuple[str, dict[str, Any]]] = []
        self._python_middlewares: List[Any] = []
        self._max_body_size: int | None = None

        self._startup_handlers: List[Callable] = []
        self._shutdown_handlers: List[Callable] = []
        self._ready_handlers: List[Callable] = []

        self._is_ready = False
        self._startup_time: float | None = None

    def set_jwt_secret(self, secret: str) -> None:
        """Set the JWT secret for authentication."""
        self._jwt_secret = secret

    def enable_logging(self, log_headers: bool = False) -> None:
        """Enable Rust logging middleware."""
        self._middlewares.append(("logging", {"log_headers": log_headers}))

    def enable_timing(self) -> None:
        """Enable Rust timing middleware."""
        self._middlewares.append(("timing", {}))

    def enable_cors(
        self,
        allow_origin: str = "*",
        allow_methods: str = "GET, POST, PUT, DELETE, PATCH, OPTIONS",
        allow_headers: str = "Content-Type, Authorization",
    ) -> None:
        """Enable Rust CORS middleware."""
        self._middlewares.append((
            "cors",
            {
                "allow_origin": allow_origin,
                "allow_methods": allow_methods,
                "allow_headers": allow_headers,
            }
        ))

    def enable_rate_limit(self, capacity: int = 100, refill_per_sec: int = 100) -> None:
        """Enable Rust rate limit middleware."""
        self._middlewares.append(("rate_limit", {"capacity": capacity, "refill_per_sec": refill_per_sec}))

    def set_body_limit(self, bytes: int) -> None:
        """Set max request body size (bytes)."""
        self._max_body_size = bytes

    def use_middleware(self, middleware: Any) -> None:
        """Register a Python middleware object or function."""
        self._python_middlewares.append(middleware)

    def route(self, path: str, methods: List[str] = ["GET"], auth: bool = False):
        """Decorator to register a route."""
        def decorator(handler):
            for method in methods:
                self._routes.append(Route(method.upper(), path, handler, auth))
            return handler
        return decorator

    def get(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("GET", path, handler, auth))
            return handler
        return self.route(path, ["GET"], auth)

    def post(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("POST", path, handler, auth))
            return handler
        return self.route(path, ["POST"], auth)

    def put(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("PUT", path, handler, auth))
            return handler
        return self.route(path, ["PUT"], auth)

    def delete(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("DELETE", path, handler, auth))
            return handler
        return self.route(path, ["DELETE"], auth)

    def patch(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("PATCH", path, handler, auth))
            return handler
        return self.route(path, ["PATCH"], auth)

    def head(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("HEAD", path, handler, auth))
            return handler
        return self.route(path, ["HEAD"], auth)

    def options(self, path: str, handler: Callable | None = None, auth: bool = False):
        if handler:
            self._routes.append(Route("OPTIONS", path, handler, auth))
            return handler
        return self.route(path, ["OPTIONS"], auth)

    def on_startup(self, func: Callable) -> Callable:
        """
        Decorator to register a startup handler.

        Startup handlers are called before the server starts accepting requests.
        They can be sync or async functions.

        Example:
            @app.on_startup
            async def init_database():
                await db.connect()
        """
        self._startup_handlers.append(func)
        return func

    def on_shutdown(self, func: Callable) -> Callable:
        """
        Decorator to register a shutdown handler.

        Shutdown handlers are called when the server is stopping.
        They can be sync or async functions.

        Example:
            @app.on_shutdown
            async def cleanup():
                await db.disconnect()
        """
        self._shutdown_handlers.append(func)
        return func

    def on_ready(self, func: Callable) -> Callable:
        """
        Decorator to register a ready handler.

        Ready handlers are called after startup is complete,
        just before the server starts accepting requests.

        Example:
            @app.on_ready
            def log_ready():
                print("Server is ready!")
        """
        self._ready_handlers.append(func)
        return func

    async def _execute_handlers(self, handlers: List[Callable]) -> None:
        """Execute a list of handlers (sync or async)."""
        import asyncio
        import inspect

        for handler in handlers:
            if inspect.iscoroutinefunction(handler):
                await handler()
            else:
                handler()

    @property
    def is_ready(self) -> bool:
        """Check if the application is ready to accept requests."""
        return self._is_ready

    @property
    def uptime(self) -> float | None:
        """Get application uptime in seconds."""
        import time
        if self._startup_time is None:
            return None
        return time.time() - self._startup_time

    def register_controller(self, controller_cls: Type) -> None:
        """
        Register a Controller class.
        Instantiates the controller and registers all its routes.
        """
        meta: ControllerMeta | None = getattr(controller_cls, "_controller_meta", None)
        if not meta:
            raise ValueError(f"Class {controller_cls.__name__} is not decorated with @Controller")

        instance = controller_cls()
        self._controllers.append(instance) # Keep reference

        print(f"🎮 Registered Controller: {controller_cls.__name__} ({meta.prefix})")

        for route_meta in meta.routes:

            prefix = meta.prefix.rstrip("/")
            path = route_meta.path
            if not path.startswith("/"):
                path = "/" + path

            full_path = prefix + path
            if full_path == "": full_path = "/"

            handler_method = getattr(instance, route_meta.handler_name)

            guards = meta.guards + getattr(handler_method, "__guards__", [])

            is_protected = any(isinstance(g, AuthGuard) for g in guards)

            all_guards = (meta.guards or []) + (route_meta.guards or [])

            wrapped_handler = wrap_handler_with_di(handler_method, guards=all_guards)

            self._routes.append(Route(route_meta.method, full_path, wrapped_handler, auth=is_protected))
            print(f"   └── {route_meta.method} {full_path}")

    def register_provider(self, interface: Type[Any], provider_cls: Type[Provider]) -> None:
        """
        Register a dependency provider.
        """
        provider_instance = provider_cls()
        register_global_provider(interface, provider_instance)
        print(f"💉 Registered Provider: {interface.__name__} -> {provider_cls.__name__}")

    def _build_native_app(self):
        """Build and configure the native application."""
        try:
            from pyvectora.pyvectora_native import App as NativeApp
        except ImportError as e:
            raise RuntimeError(
                "Native module not available. Run 'maturin develop' to build."
            ) from e

        native_app = NativeApp(self.host, self.port)
        if self._jwt_secret:
            native_app.enable_auth(self._jwt_secret)
        if self._max_body_size is not None:
            native_app.set_body_limit(self._max_body_size)

        for name, cfg in self._middlewares:
            if name == "logging":
                native_app.enable_logging_middleware(cfg.get("log_headers", False))
            elif name == "timing":
                native_app.enable_timing_middleware()
            elif name == "cors":
                native_app.enable_cors_middleware(
                    cfg.get("allow_origin", "*"),
                    cfg.get("allow_methods", "GET, POST, PUT, DELETE, PATCH, OPTIONS"),
                    cfg.get("allow_headers", "Content-Type, Authorization"),
                )
            elif name == "rate_limit":
                native_app.enable_rate_limit_middleware(
                    cfg.get("capacity", 100),
                    cfg.get("refill_per_sec", 100),
                )

        for mw in self._python_middlewares:
            native_app.add_python_middleware(mw)

        from .schema import OpenAPIGenerator
        import json
        from .response import Response
        from .di import solve_dependencies

        def make_internal(handler):
            async def wrapper(py_req):
                try:
                    kwargs = await solve_dependencies(handler, py_req)
                except Exception as e:
                    print(f"DI Error details: {e}")
                    return Response.json({"error": f"Dependency Injection Failed: {e}"}, status=500)

                if asyncio.iscoroutinefunction(handler):
                    return await handler(**kwargs)
                return handler(**kwargs)
            return wrapper

        generator = OpenAPIGenerator(self)
        openapi_schema = generator.generate()

        schema_json = json.dumps(openapi_schema)

        def openapi_handler(req):
            return Response.json(json.loads(schema_json))

        docs_html = f"""
        <!DOCTYPE html>
        <html>
        <head>
        <link type="text/css" rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
        <title>PyVectora Docs</title>
        </head>
        <body>
        <div id="swagger-ui"></div>
        <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
        <script>
        SwaggerUIBundle({{
            url: '/openapi.json',
            dom_id: '#swagger-ui',
        }});
        </script>
        </body>
        </html>
        """
        def docs_handler(req):
            return Response.text(docs_html).with_status(200).with_header("Content-Type", "text/html")

        native_app.get("/openapi.json", make_internal(openapi_handler))
        native_app.get("/docs", make_internal(docs_handler))
        print("📚 Docs available at /docs")

        if self.enable_health_check:
            def health_handler(req):
                import time
                health_data = {
                    "status": "healthy" if self._is_ready else "starting",
                    "uptime_seconds": round(time.time() - self._startup_time, 2) if self._startup_time else 0,
                    "version": "0.1.0",
                }
                return Response.json(health_data)

            native_app.get("/health", make_internal(health_handler))
            print("❤️  Health check at /health")

        for route in self._routes:
            method = route.method.lower()
            handler_fn = getattr(native_app, method, None)
            if handler_fn:
                handler_fn(route.path, route.handler, auth=route.auth)

        self.native_app = native_app
        return native_app

    def serve(self):
        """Start the HTTP server with lifecycle management."""
        import time
        import asyncio

        native_app = self._build_native_app()
        print(f"🚀 Serving on {self.host}:{self.port}")

        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)

        if self._startup_handlers:
            print("🔄 Running startup handlers...")
            try:
                loop.run_until_complete(self._execute_handlers(self._startup_handlers))
                print(f"✅ {len(self._startup_handlers)} startup handler(s) completed")
            except Exception as e:
                print(f"❌ Startup handler failed: {e}")
                raise

        ctx = None
        if self.lifespan:
            ctx = self.lifespan(self)
            try:
                loop.run_until_complete(ctx.__aenter__())
                print("✅ Lifespan: Startup complete")
            except Exception as e:
                print(f"❌ Lifespan startup failed: {e}")
                raise

        self._startup_time = time.time()
        self._is_ready = True

        if self._ready_handlers:
            try:
                loop.run_until_complete(self._execute_handlers(self._ready_handlers))
            except Exception as e:
                print(f"⚠️  Ready handler error: {e}")

        print("🟢 Server ready to accept connections")

        async def bootstrap():
            await native_app.serve()

        try:
            loop.run_until_complete(bootstrap())
        except KeyboardInterrupt:
            print("\n🛑 Shutdown signal received")
        finally:

            self._is_ready = False

            if self._shutdown_handlers:
                print("🔄 Running shutdown handlers...")
                try:
                    loop.run_until_complete(
                        self._execute_handlers(list(reversed(self._shutdown_handlers)))
                    )
                    print(f"✅ {len(self._shutdown_handlers)} shutdown handler(s) completed")
                except Exception as e:
                    print(f"❌ Shutdown handler error: {e}")

            if ctx:
                try:
                    loop.run_until_complete(ctx.__aexit__(None, None, None))
                except Exception as e:
                    print(f"❌ Lifespan shutdown error: {e}")

            print("👋 Server stopped")

    def test_client(self):
        """Return a Zero-Network TestClient for this app."""
        from .test_client import TestClient
        self._build_native_app()
        return TestClient(self)
