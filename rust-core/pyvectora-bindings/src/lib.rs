//! # PyVectora Python Bindings
//!
//! PyO3-based Python extension module for PyVectora.
//! Exposes the Rust runtime to Python with ergonomic APIs.
//!
//! ## Architecture
//!
//! This module follows the "Inversion of Control" pattern:
//! - Python calls `pyvectora.serve()` to start the server
//! - Control transfers to Rust's Tokio runtime
//! - Python handlers are called as callbacks from Rust

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyBytes};
use pyo3::exceptions::{PyStopIteration, PyStopAsyncIteration};
use pyvectora_core::router::Method;
use pyvectora_core::server::{PyRequest as RustRequest, PyResponse as RustResponse, Server, Handler};
use pyvectora_core::middleware::{LoggingMiddleware, TimingMiddleware, CorsMiddleware, RateLimitMiddleware};
use pyvectora_core::middleware::{Middleware, MiddlewareResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, OnceLock};
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;
use tracing::warn;
use tokio_util::sync::CancellationToken;

mod error;
mod database;

use error::register_exceptions;
use pyvectora_core::PyRequest;
mod context;
use context::PyExecutionContext;
use database::register_database_classes;

/// Global Tokio runtime for test client operations
///
/// Lazily initialized on first use, shared across all test requests.
static GLOBAL_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or create the global Tokio runtime
///
/// Thread-safe, lock-free after first initialization.
/// Made public for database module access.
pub(crate) fn get_runtime() -> &'static Runtime {
    GLOBAL_RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Initialize tracing for the library
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("pyvectora=info".parse().unwrap()))
        .json()
        .try_init();
}

/// Python-exposed Request object
///
/// Contains request data with typed params (FAZ 2) and
/// headers/query access (FAZ 3).

/// Convert JSON value to Python object
fn json_to_pyobject(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    Ok(match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.to_object(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_object(py)
            } else if let Some(f) = n.as_f64() {
                f.to_object(py)
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => s.to_object(py),
        serde_json::Value::Array(arr) => {
            let list = pyo3::types::PyList::empty(py);
            for item in arr {
                list.append(json_to_pyobject(py, item)?)?;
            }
            list.into()
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_pyobject(py, v)?)?;
            }
            dict.into()
        }
    })
}

/// Check if a Python object is a coroutine (async result)
///
/// Uses `inspect.iscoroutine()` to detect async handler results.
fn is_coroutine(py: Python<'_>, obj: &PyObject) -> bool {
    py.import("inspect")
        .and_then(|inspect| inspect.getattr("iscoroutine"))
        .and_then(|iscoroutine| iscoroutine.call1((obj,)))
        .and_then(|result| result.extract::<bool>())
        .unwrap_or(false)
}

/// Run a Python coroutine using asyncio.run()
///
/// This executes an async handler synchronously by running
/// it on Python's asyncio event loop.
fn run_coroutine(py: Python<'_>, coro: &PyObject) -> PyResult<PyObject> {
    let asyncio = py.import("asyncio")?;
    let run = asyncio.getattr("run")?;
    let result = run.call1((coro,))?;
    Ok(result.into())
}

/// Python-exposed Response object
#[pyclass(name = "Response")]
#[derive(Clone)]
pub struct PyResponse {
    #[pyo3(get, set)]
    status: u16,
    #[pyo3(get, set)]
    body: String,
    #[pyo3(get, set)]
    content_type: String,
    #[pyo3(get, set)]
    headers: HashMap<String, String>,
}

#[pymethods]
impl PyResponse {
    #[new]
    #[pyo3(signature = (body="", status=200, content_type="application/json"))]
    fn new(body: &str, status: u16, content_type: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
            content_type: content_type.to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a JSON response
    #[staticmethod]
    #[pyo3(signature = (data, status=200))]
    fn json(py: Python<'_>, data: &PyAny, status: u16) -> PyResult<Self> {
        let json_str = if data.is_instance_of::<PyDict>() {
            let json_module = py.import("json")?;
            let dumps = json_module.getattr("dumps")?;
            dumps.call1((data,))?.extract::<String>()?
        } else if data.is_instance_of::<PyString>() {
            data.extract::<String>()?
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "data must be a dict or string",
            ));
        };

        Ok(Self {
            status,
            body: json_str,
            content_type: "application/json".to_string(),
            headers: HashMap::new(),
        })
    }

    /// Set status code (builder pattern)
    fn with_status<'a>(mut slf: PyRefMut<'a, Self>, status: u16) -> PyRefMut<'a, Self> {
        slf.status = status;
        slf
    }

    /// Set header (builder pattern)
    /// Currently only supports Content-Type for MVP
    fn with_header<'a>(mut slf: PyRefMut<'a, Self>, key: &str, value: &str) -> PyRefMut<'a, Self> {
        if key.eq_ignore_ascii_case("content-type") {
            slf.content_type = value.to_string();
        } else {
            slf.headers.insert(key.to_string(), value.to_string());
        }
        slf
    }

    /// Create a text response
    #[staticmethod]
    #[pyo3(signature = (text, status=200))]
    fn text(text: &str, status: u16) -> Self {
        Self {
            status,
            body: text.to_string(),
            content_type: "text/plain".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Route registration for the App
struct Route {
    method: Method,
    path: String,
    handler: PyObject,
    auth: bool,
}

#[derive(Clone)]
enum MiddlewareConfig {
    Logging { log_headers: bool },
    Timing,
    Cors { allow_origin: String, allow_methods: String, allow_headers: String },
    RateLimit { capacity: u64, refill_per_sec: u64 },
}

/// Python-exposed App object
#[pyclass(name = "App")]
pub struct PyApp {
    routes: Vec<Route>,
    host: String,
    port: u16,
    /// Application state (Python objects)
    /// Application state (Python objects)
    state: Arc<RwLock<HashMap<String, PyObject>>>,
    /// JWT Secret for authentication
    jwt_secret: Option<String>,
    /// Middleware configuration
    middlewares: Vec<MiddlewareConfig>,
    /// Max request body size
    max_body_size: usize,
    /// Python middleware objects
    python_middlewares: Vec<PyObject>,
}

#[pymethods]
impl PyApp {
    #[new]
    #[pyo3(signature = (host="127.0.0.1", port=8000))]
    fn new(host: &str, port: u16) -> Self {
        Self {
            routes: Vec::new(),
            host: host.to_string(),
            port,
            state: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret: None,
            middlewares: Vec::new(),
            max_body_size: 1024 * 1024,
            python_middlewares: Vec::new(),
        }
    }

    /// Enable JWT authentication
    fn enable_auth(&mut self, secret: &str) {
        self.jwt_secret = Some(secret.to_string());
    }

    /// Get all state as a dict
    fn get_all_state(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        let state = self.state.read().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("State lock poisoned")
        })?;
        for (k, v) in state.iter() {
            dict.set_item(k, v.clone_ref(py))?;
        }
        Ok(dict.into())
    }

    /// Set a state value
    fn set_state(&self, _py: Python<'_>, key: &str, value: &PyAny) -> PyResult<()> {
        let mut state = self.state.write().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("State lock poisoned")
        })?;
        state.insert(key.to_string(), value.into());
        Ok(())
    }

    /// Get a state value by key
    fn state_get(&self, py: Python<'_>, key: &str) -> PyResult<Option<PyObject>> {
        let state = self.state.read().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("State lock poisoned")
        })?;
        Ok(state.get(key).map(|v| v.clone_ref(py)))
    }

    /// Register a GET route
    #[pyo3(signature = (path, handler, auth=false))]
    fn get(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Get,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register a POST route
    #[pyo3(signature = (path, handler, auth=false))]
    fn post(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Post,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register a PUT route
    #[pyo3(signature = (path, handler, auth=false))]
    fn put(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Put,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register a DELETE route
    #[pyo3(signature = (path, handler, auth=false))]
    fn delete(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Delete,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register a PATCH route
    #[pyo3(signature = (path, handler, auth=false))]
    fn patch(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Patch,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register a HEAD route
    #[pyo3(signature = (path, handler, auth=false))]
    fn head(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Head,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Register an OPTIONS route
    #[pyo3(signature = (path, handler, auth=false))]
    fn options(&mut self, path: &str, handler: PyObject, auth: bool) {
        self.routes.push(Route {
            method: Method::Options,
            path: path.to_string(),
            handler,
            auth,
        });
    }

    /// Enable logging middleware
    #[pyo3(signature = (log_headers=false))]
    fn enable_logging_middleware(&mut self, log_headers: bool) {
        self.middlewares.push(MiddlewareConfig::Logging { log_headers });
    }

    /// Enable timing middleware
    fn enable_timing_middleware(&mut self) {
        self.middlewares.push(MiddlewareConfig::Timing);
    }

    /// Enable CORS middleware
    #[pyo3(signature = (allow_origin="*", allow_methods="GET, POST, PUT, DELETE, PATCH, OPTIONS", allow_headers="Content-Type, Authorization"))]
    fn enable_cors_middleware(&mut self, allow_origin: &str, allow_methods: &str, allow_headers: &str) {
        self.middlewares.push(MiddlewareConfig::Cors {
            allow_origin: allow_origin.to_string(),
            allow_methods: allow_methods.to_string(),
            allow_headers: allow_headers.to_string(),
        });
    }

    /// Enable rate limit middleware
    #[pyo3(signature = (capacity=100, refill_per_sec=100))]
    fn enable_rate_limit_middleware(&mut self, capacity: u64, refill_per_sec: u64) {
        self.middlewares.push(MiddlewareConfig::RateLimit { capacity, refill_per_sec });
    }

    /// Set max request body size (bytes)
    fn set_body_limit(&mut self, bytes: usize) {
        self.max_body_size = bytes;
    }

    /// Register a Python middleware object or function
    fn add_python_middleware(&mut self, middleware: PyObject) {
        self.python_middlewares.push(middleware);
    }
    /// Start the server (blocks until shutdown)
    /// Start the server (returns awaitable future)
    /// Start the server (returns awaitable future)
    fn serve<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        init_tracing();

        let host = self.host.clone();
        let port = self.port;
        let jwt_secret = self.jwt_secret.clone();
        let middleware_data = self.middlewares.clone();
        let python_middleware_data: Vec<PyObject> = self.python_middlewares
            .iter()
            .map(|m| m.clone_ref(py))
            .collect();
        let max_body_size = self.max_body_size;

        struct RouteData {
            method: Method,
            path: String,
            handler: PyObject,
            auth: bool,
        }

        let route_data: Vec<RouteData> = self.routes.iter().map(|r| RouteData {
            method: r.method,
            path: r.path.clone(),
            handler: r.handler.clone_ref(py),
            auth: r.auth,
        }).collect();

        init_asyncio_once(py)?;

        let event_loop = py.import("asyncio")?.call_method0("get_running_loop")?;
        let locals = pyo3_asyncio::TaskLocals::new(event_loop);

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let addr: std::net::SocketAddr = format!("{}:{}", host, port)
                .parse()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;

            let mut server = Server::new(jwt_secret.as_deref().unwrap_or(""));
            server = server.bind(addr);
            if let Some(secret) = &jwt_secret {
                server.enable_auth(secret);
            }
            server.set_max_body_size(max_body_size);
            apply_middlewares(&mut server, &middleware_data);
            apply_python_middlewares(&mut server, &python_middleware_data, locals.clone());

            for route in route_data {
                let rust_handler = create_handler_adapter(route.handler, locals.clone());
                server.add_route(route.method, &route.path, rust_handler, route.auth)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            }

            server.serve().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            Ok(())
        })
    }

    /// Create a test client (zero-network)
    fn test_client(&self, py: Python<'_>) -> PyResult<PyServer> {
        let jwt_secret = self.jwt_secret.clone();
        let middleware_data = self.middlewares.clone();
        let python_middleware_data: Vec<PyObject> = self.python_middlewares
            .iter()
            .map(|m| m.clone_ref(py))
            .collect();
        let max_body_size = self.max_body_size;

        struct RouteData {
            method: Method,
            path: String,
            handler: PyObject,
            auth: bool,
        }

        let route_data: Vec<RouteData> = self.routes.iter().map(|r| RouteData {
            method: r.method,
            path: r.path.clone(),
            handler: r.handler.clone_ref(py),
            auth: r.auth,
        }).collect();

        init_asyncio_once(py)?;

        let asyncio = py.import("asyncio")?;
        let event_loop = match asyncio.call_method0("get_running_loop") {
            Ok(loop_) => loop_,
            Err(_) => {
                let policy = asyncio.call_method0("get_event_loop_policy")?;
                let new_loop = policy.call_method0("new_event_loop")?;
                policy.call_method1("set_event_loop", (new_loop.clone(),))?;
                new_loop
            }
        };
        let locals = pyo3_asyncio::TaskLocals::new(event_loop);

        let mut server = Server::new(jwt_secret.as_deref().unwrap_or(""));
        if let Some(secret) = &jwt_secret {
            server.enable_auth(secret);
        }
        server.set_max_body_size(max_body_size);
        apply_middlewares(&mut server, &middleware_data);
        apply_python_middlewares(&mut server, &python_middleware_data, locals.clone());

        for route in route_data {
            let rust_handler = create_handler_adapter(route.handler, locals.clone());
            server.add_route(route.method, &route.path, rust_handler, route.auth)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        }

        Ok(PyServer { inner: server })
    }
}

static INIT_ASYNCIO: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn init_asyncio_once(_py: Python<'_>) -> PyResult<()> {
    INIT_ASYNCIO.get_or_init(|| {
        let _ = pyo3_asyncio::tokio::get_runtime();
    });
    Ok(())
}

fn build_tokio_runtime() -> PyResult<Runtime> {
    Runtime::new()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

fn apply_middlewares(server: &mut Server, configs: &[MiddlewareConfig]) {
    for cfg in configs {
        match cfg {
            MiddlewareConfig::Logging { log_headers } => {
                let mut mw = LoggingMiddleware::new();
                if *log_headers {
                    mw = mw.with_headers();
                }
                server.add_middleware(mw);
            }
            MiddlewareConfig::Timing => {
                server.add_middleware(TimingMiddleware::new());
            }
            MiddlewareConfig::Cors { allow_origin, allow_methods, allow_headers } => {
                let mw = CorsMiddleware::new()
                    .allow_origin(allow_origin.clone())
                    .allow_methods(allow_methods.clone())
                    .allow_headers(allow_headers.clone());
                server.add_middleware(mw);
            }
            MiddlewareConfig::RateLimit { capacity, refill_per_sec } => {
                server.add_middleware(RateLimitMiddleware::new(*capacity, *refill_per_sec));
            }
        }
    }
}

struct PythonMiddleware {
    inner: PyObject,
    locals: pyo3_asyncio::TaskLocals,
}

impl PythonMiddleware {
    fn new(inner: PyObject, locals: pyo3_asyncio::TaskLocals) -> Self {
        Self { inner, locals }
    }

    fn before(&self, req: &RustRequest) -> Result<Option<RustResponse>, PyErr> {
        Python::with_gil(|py| {
            let callable = {
                let any = self.inner.as_ref(py);
                if any.hasattr("before_request")? {
                    Some(any.getattr("before_request")?.into())
                } else if any.is_callable() {
                    Some(self.inner.clone_ref(py))
                } else {
                    None
                }
            };
            let callable = match callable {
                Some(c) => c,
                None => return Ok(None),
            };
            let py_req = req.clone().into_py(py);
            let result = callable.call1(py, (py_req,))?;
            let obj = result.to_object(py);
            if is_coroutine(py, &obj) {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>("Middleware must be sync"));
            }
            if result.is_none(py) {
                Ok(None)
            } else {
                Ok(Some(convert_python_response(py, obj)))
            }
        })
    }

    fn after(&self, req: &RustRequest, res: &RustResponse) -> Result<Option<RustResponse>, PyErr> {
        Python::with_gil(|py| {
            let callable = match select_callable(py, &self.inner, "after_response") {
                Ok(c) => c,
                Err(_) => return Ok(None),
            };
            let py_req = req.clone().into_py(py);
            let py_res = rust_response_to_py(py, res)?;
            let result = callable.call1(py, (py_req, py_res))?;
            let obj = result.to_object(py);
            if is_coroutine(py, &obj) {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>("Middleware must be sync"));
            }
            if result.is_none(py) {
                Ok(None)
            } else {
                Ok(Some(convert_python_response(py, obj)))
            }
        })
    }
}

impl Middleware for PythonMiddleware {
    fn before_request(&self, req: &RustRequest) -> MiddlewareResult {
        match self.before(req) {
            Ok(Some(resp)) => MiddlewareResult::Respond(resp),
            Ok(None) => MiddlewareResult::Continue,
            Err(err) => MiddlewareResult::Respond(convert_py_error(err)),
        }
    }

    fn after_response(&self, req: &RustRequest, res: &mut RustResponse) {
        match self.after(req, res) {
            Ok(Some(new_resp)) => *res = new_resp,
            Ok(None) => {}
            Err(err) => {
                *res = convert_py_error(err);
            }
        }
    }

    fn name(&self) -> &'static str {
        "PythonMiddleware"
    }
}

fn select_callable(py: Python<'_>, target: &PyObject, method: &str) -> Result<PyObject, PyErr> {
    let any = target.as_ref(py);
    if any.hasattr(method)? {
        Ok(any.getattr(method)?.into())
    } else if any.is_callable() {
        Ok(target.clone_ref(py))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>("Middleware is not callable"))
    }
}

fn rust_response_to_py(py: Python<'_>, res: &RustResponse) -> PyResult<PyObject> {
    let mut py_resp = PyResponse::new(&res.body, res.status, &res.content_type);
    py_resp.headers = res.headers.clone();
    let py_resp = Py::new(py, py_resp)?;
    Ok(py_resp.to_object(py))
}

fn apply_python_middlewares(server: &mut Server, items: &[PyObject], locals: pyo3_asyncio::TaskLocals) {
    Python::with_gil(|py| {
        for item in items {
            server.add_middleware(PythonMiddleware::new(item.clone_ref(py), locals.clone()));
        }
    });
}

impl From<RustResponse> for PyResponse {
    fn from(r: RustResponse) -> Self {
        let mut resp = PyResponse::new(&r.body, r.status, &r.content_type);
        resp.headers = r.headers;
        resp
    }
}

/// Adapt Python handler to Rust Core handler with panic safety
///
/// This is the critical FFI boundary - all panics MUST be caught here
/// to prevent crashing the Python interpreter.
fn create_handler_adapter(handler: PyObject, locals: pyo3_asyncio::TaskLocals) -> Handler {
    Arc::new(move |req, _matched| {
        let handler = handler.clone();
        let locals = locals.clone();
        let req = req.clone();
        let token = CancellationToken::new();
        let ctx = PyExecutionContext::new(token.clone());

        Box::pin(async move {
            execute_handler(handler, ctx, req, locals).await
        })
    })
}

fn is_coroutine_function(handler: &PyObject) -> bool {
    Python::with_gil(|py| {
        let inspect = py.import("inspect").ok();
        if let Some(inspect) = inspect {
             inspect.call_method1("iscoroutinefunction", (handler,))
                .map(|res| res.is_true().unwrap_or(false))
                .unwrap_or(false)
        } else {
            false
        }
    })
}

fn convert_py_error(err: PyErr) -> RustResponse {
    Python::with_gil(|py| {
        err.print(py);
        let error_msg = err.to_string().replace('"', "\\\"");
        RustResponse::json(format!(
            r#"{{"error": "Internal Server Error", "details": "{}"}}"#,
            error_msg
        )).with_status(500)
    })
}

async fn execute_handler(
    handler: PyObject,
    ctx: PyExecutionContext,
    req: RustRequest,
    locals: pyo3_asyncio::TaskLocals,
) -> RustResponse {
    let is_async = is_coroutine_function(&handler);

    let fut_result = Python::with_gil(|py| -> PyResult<std::pin::Pin<Box<dyn std::future::Future<Output = PyResult<PyObject>> + Send>>> {
        if is_async {
            let py_req = req.clone().into_py(py);
            let py_ctx = Py::new(py, ctx)?;
            py_req.as_ref(py).setattr("context", py_ctx)?;

            let coro = handler.call1(py, (py_req,))?;
            let fut = pyo3_asyncio::into_future_with_locals(&locals, coro.as_ref(py))?;
            Ok(Box::pin(fut))
        } else {
            let py_req = req.clone().into_py(py);
            let py_ctx = Py::new(py, ctx)?;
            py_req.as_ref(py).setattr("context", py_ctx)?;

            let resp = handler.call1(py, (py_req,))?;
            Ok(Box::pin(std::future::ready(Ok(resp))))
        }
    });

    let result = match fut_result {
        Ok(fut) => fut.await,
        Err(e) => Err(e),
    };

    match result {
         Ok(py_resp) => {
             if Python::with_gil(|py| is_streaming_response(py, &py_resp)) {
                 collect_streaming_response(py_resp, &locals).await
             } else {
                 Python::with_gil(|py| convert_python_response(py, py_resp))
             }
         }
         Err(e) => convert_py_error(e),
    }
}

/// Convert Python response object to Rust response
///
/// OPTIMIZATION: Fast path for PyResponse, minimal Python calls for other types.
#[inline]
fn convert_python_response(py: Python<'_>, result: PyObject) -> RustResponse {
    if let Ok(resp) = result.extract::<PyResponse>(py) {
        return RustResponse {
            status: resp.status,
            body: resp.body,
            content_type: resp.content_type,
            headers: resp.headers,
        };
    }

    if let Ok(text) = result.extract::<String>(py) {
        return RustResponse::text(text);
    }

    let bound = result.as_ref(py);
    if let Ok(status_attr) = bound.getattr("status") {
        let status = status_attr.extract::<u16>().unwrap_or(200);
        let body = bound
            .getattr("body")
            .and_then(|b| b.extract::<String>())
            .unwrap_or_default();
        let content_type = bound
            .getattr("content_type")
            .and_then(|ct| ct.extract::<String>())
            .unwrap_or_else(|_| "application/json".to_string());
        let headers = bound
            .getattr("headers")
            .and_then(|h| h.extract::<HashMap<String, String>>())
            .unwrap_or_default();
        return RustResponse {
            status,
            body,
            content_type,
            headers,
        };
    }

    if let Ok(dict) = result.downcast::<PyDict>(py) {
        if let Ok(json_module) = py.import("json") {
            if let Ok(dumps) = json_module.getattr("dumps") {
                if let Ok(json_result) = dumps.call1((dict,)) {
                    if let Ok(json_str) = json_result.extract::<String>() {
                        return RustResponse::json(json_str);
                    }
                }
            }
        }
        return RustResponse::json("{}".to_string());
    }

    RustResponse::text("Internal Server Error: Unsupported response type")
        .with_status(500)
}

fn is_streaming_response(py: Python<'_>, result: &PyObject) -> bool {
    result
        .as_ref(py)
        .getattr("_is_streaming")
        .and_then(|v| v.extract::<bool>())
        .unwrap_or(false)
}

async fn collect_streaming_response(
    result: PyObject,
    locals: &pyo3_asyncio::TaskLocals,
) -> RustResponse {
    let (status, content_type, headers, content) = match Python::with_gil(|py| {
        let resp = result.as_ref(py);
        let status = resp.getattr("status").and_then(|v| v.extract::<u16>()).unwrap_or(200);
        let content_type = resp.getattr("content_type")
            .and_then(|v| v.extract::<String>())
            .unwrap_or_else(|_| "text/plain".to_string());
        let headers = resp.getattr("headers")
            .and_then(|h| h.extract::<HashMap<String, String>>())
            .unwrap_or_default();
        let mut content = resp.getattr("content")?;
        if content.is_callable() {
            content = content.call0()?;
        }
        Ok((status, content_type, headers, content.into_py(py)))
    }) {
        Ok(v) => v,
        Err(err) => return convert_py_error(err),
    };

    let mut out = String::new();

    let is_async = Python::with_gil(|py| {
        let any = content.as_ref(py);
        any.hasattr("__anext__").unwrap_or(false) || any.hasattr("__aiter__").unwrap_or(false)
    });

    if is_async {
        let async_iter = Python::with_gil(|py| content.as_ref(py).call_method0("__aiter__").map(|v| v.into_py(py)));
        let async_iter = match async_iter {
            Ok(v) => v,
            Err(err) => return convert_py_error(err),
        };
        loop {
            let fut = Python::with_gil(|py| -> PyResult<_> {
                let anext = async_iter.as_ref(py).call_method0("__anext__")?;
                let fut = pyo3_asyncio::into_future_with_locals(locals, anext)?;
                Ok(fut)
            });
            let next = match fut {
                Ok(fut) => fut.await,
                Err(err) => return convert_py_error(err),
            };
            match next {
                Ok(item) => {
                    if let Ok(chunk) = Python::with_gil(|py| py_chunk_to_string(py, item)) {
                        out.push_str(&chunk);
                    }
                }
                Err(err) => {
                    let is_stop = Python::with_gil(|py| err.is_instance_of::<PyStopAsyncIteration>(py));
                    if is_stop {
                        break;
                    }
                    return convert_py_error(err);
                }
            }
        }
    } else {
        let iter = Python::with_gil(|py| content.as_ref(py).call_method0("__iter__").map(|v| v.into_py(py)));
        let iter = match iter {
            Ok(v) => v,
            Err(err) => return convert_py_error(err),
        };
        loop {
            let next = Python::with_gil(|py| -> PyResult<Option<PyObject>> {
                match iter.as_ref(py).call_method0("__next__") {
                    Ok(item) => Ok(Some(item.into())),
                    Err(err) => {
                        if err.is_instance_of::<PyStopIteration>(py) {
                            Ok(None)
                        } else {
                            Err(err)
                        }
                    }
                }
            });
            match next {
                Ok(Some(item)) => {
                    if let Ok(chunk) = Python::with_gil(|py| py_chunk_to_string(py, item)) {
                        out.push_str(&chunk);
                    }
                }
                Ok(None) => break,
                Err(err) => return convert_py_error(err),
            }
        }
    }

    RustResponse {
        status,
        body: out,
        content_type,
        headers,
    }
}

fn py_chunk_to_string(py: Python<'_>, obj: PyObject) -> PyResult<String> {
    let any = obj.as_ref(py);
    if let Ok(b) = any.downcast::<PyBytes>() {
        return Ok(String::from_utf8_lossy(b.as_bytes()).to_string());
    }
    if let Ok(s) = any.downcast::<PyString>() {
        return Ok(s.to_str()?.to_string());
    }
    Ok(any.str()?.to_str()?.to_string())
}
/// Server wrapper for zero-network testing
#[pyclass(name = "Server")]
struct PyServer {
    inner: Server,
}

#[pymethods]
impl PyServer {
    /// Execute a test request directly (bypassing TCP)
    #[pyo3(signature = (method, path, headers=None, body=None))]
    fn test_request<'py>(
        &self,
        _py: Python<'py>,
        method: &str,
        path: String,
        headers: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
    ) -> PyResponse {
        let method = match method.to_uppercase().as_str() {
             "GET" => pyvectora_core::router::Method::Get,
             "POST" => pyvectora_core::router::Method::Post,
             "PUT" => pyvectora_core::router::Method::Put,
             "DELETE" => pyvectora_core::router::Method::Delete,
             "PATCH" => pyvectora_core::router::Method::Patch,
             "HEAD" => pyvectora_core::router::Method::Head,
             "OPTIONS" => pyvectora_core::router::Method::Options,
             _ => pyvectora_core::router::Method::Get,
        };

        let headers_map = headers.unwrap_or_default();

        let body_bytes = body.map(pyvectora_core::server::Bytes::from);

        let rt = get_runtime();
        let resp = rt.block_on(self.inner.test_request(
             method, path, headers_map, body_bytes
        ));

        PyResponse::from(resp)
    }
}

/// Library version
#[pyfunction]
fn version() -> &'static str {
    pyvectora_core::VERSION
}

/// PyVectora Python module
#[pymodule]
fn pyvectora_native(_py: Python, m: &PyModule) -> PyResult<()> {
    register_exceptions(m)?;

    m.add_class::<PyApp>()?;
    m.add_class::<PyRequest>()?;
    m.add_class::<PyResponse>()?;
    m.add_class::<PyServer>()?;

    register_database_classes(m)?;

    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
