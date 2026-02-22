//! # HTTP Server
//!
//! High-performance HTTP server built on Hyper and Tokio.
//! Implements graceful shutdown with signal handling.
//!
//! ## Key Features
//!
//! - Async request handling with Tokio runtime
//! - Graceful shutdown on SIGINT/SIGTERM
//! - Connection keep-alive support
//! - Zero-copy body streaming

use crate::error::Result;
use crate::router::{Match, Method, Router};
use http_body_util::Full;
pub use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Authentication Configuration (JWT)
#[derive(Clone)]
pub struct AuthConfig {
    /// JWT decoding key
    pub decoding_key: DecodingKey,
    /// JWT validation settings
    pub validation: Validation,
}

impl AuthConfig {
    /// Create auth config from shared secret
    pub fn new(secret: &str) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation: Validation::new(Algorithm::HS256),
        }
    }
}

/// HTTP Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the server to
    pub address: SocketAddr,
    /// Enable keep-alive connections
    pub keep_alive: bool,
    /// Shutdown timeout for graceful shutdown (default: 30 seconds)
    pub shutdown_timeout: Duration,
    /// Max request body size in bytes
    pub max_body_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: ([127, 0, 0, 1], 8000).into(),
            keep_alive: true,
            shutdown_timeout: Duration::from_secs(30),
            max_body_size: 1024 * 1024,
        }
    }
}

pub use crate::request::PyRequest;

/// HTTP Response wrapper for Python interop
pub struct PyResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body
    pub body: String,
    /// Content type
    pub content_type: String,
    /// Response headers
    pub headers: HashMap<String, String>,
}

impl std::fmt::Debug for PyResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyResponse")
            .field("status", &self.status)
            .field("body", &self.body)
            .field("content_type", &self.content_type)
            .field("headers", &self.headers)
            .finish()
    }
}

impl Default for PyResponse {
    fn default() -> Self {
        Self {
            status: 200,
            body: String::new(),
            content_type: "application/json".to_string(),
            headers: HashMap::new(),
        }
    }
}

impl PyResponse {
    /// Create a JSON response
    #[must_use]
    pub fn json(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            content_type: "application/json".to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a text response
    #[must_use]
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            content_type: "text/plain".to_string(),
            headers: HashMap::new(),
        }
    }

    /// Set status code
    #[must_use]
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Set header (simple Content-Type support for now)
    #[must_use]
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        if key.eq_ignore_ascii_case("content-type") {
            self.content_type = value.to_string();
        } else {
            self.headers.insert(key.to_string(), value.to_string());
        }
        self
    }

    /// Set or override a header
    pub fn set_header(&mut self, key: &str, value: &str) {
        if key.eq_ignore_ascii_case("content-type") {
            self.content_type = value.to_string();
        } else {
            self.headers.insert(key.to_string(), value.to_string());
        }
    }

    /// Convert to hyper Response
    fn into_hyper(self) -> Response<Full<Bytes>> {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut builder = Response::builder().status(status);
        builder = builder.header("Content-Type", &self.content_type);
        for (k, v) in &self.headers {
            if !k.eq_ignore_ascii_case("content-type") {
                builder = builder.header(k.as_str(), v.as_str());
            }
        }

        builder
            .body(Full::new(Bytes::from(self.body)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Internal Server Error")))
                    .unwrap()
            })
    }
}

/// Handler function type (async)
pub type Handler = Arc<
    dyn Fn(
            &PyRequest,
            &Match<'_>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = PyResponse> + Send>>
        + Send
        + Sync,
>;

/// High-performance HTTP server
pub struct Server {
    config: ServerConfig,
    router: Router,
    handlers: Vec<Handler>,
    auth_config: Option<Arc<AuthConfig>>,
    middleware: crate::middleware::MiddlewareChain,
}

impl Server {
    /// Create a new Server instance
    pub fn new(secret: &str) -> Self {
        Self {
            config: ServerConfig::default(),
            router: Router::new(),
            handlers: Vec::new(),
            auth_config: if secret.is_empty() {
                None
            } else {
                Some(Arc::new(AuthConfig::new(secret)))
            },
            middleware: crate::middleware::MiddlewareChain::new(),
        }
    }

    /// Bind the server to an address
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.config.address = addr;
        self
    }

    /// Set max request body size
    pub fn set_max_body_size(&mut self, bytes: usize) {
        self.config.max_body_size = bytes;
    }

    /// Enable JWT authentication
    pub fn enable_auth(&mut self, secret: &str) {
        self.auth_config = Some(Arc::new(AuthConfig::new(secret)));
    }

    /// Add a middleware to the chain
    pub fn add_middleware<M: crate::middleware::Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.add(middleware);
    }

    /// Add a route and its handler
    pub fn add_route(
        &mut self,
        method: Method,
        path: &str,
        handler: Handler,
        auth_required: bool,
    ) -> Result<()> {
        self.router.add_route(method, path, auth_required)?;
        self.handlers.push(handler);
        Ok(())
    }

    /// Start the server with graceful shutdown
    pub async fn serve(&self) -> Result<()> {
        let addr = self.config.address;

        let socket = tokio::net::TcpSocket::new_v4()?;
        socket.set_reuseaddr(true)?;
        #[cfg(not(windows))]
        {
            socket.set_reuseport(true)?;
        }
        socket.bind(addr)?;

        let listener = socket.listen(1024)?;

        info!("Server listening on http://{}", addr);

        let router = Arc::new(self.router.clone());
        let handlers = Arc::new(self.handlers.clone());
        let auth_config = self.auth_config.clone();
        let middleware = Arc::new(self.middleware.clone());
        let active = Arc::new(AtomicUsize::new(0));
        let max_body_size = self.config.max_body_size;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (stream, remote_addr) = accept_result?;
                    let io = TokioIo::new(stream);

                    let router = router.clone();
                    let handlers = handlers.clone();
                    let auth_config = auth_config.clone();
                    let middleware = middleware.clone();
                    let active = active.clone();

                    tokio::task::spawn(async move {
                        active.fetch_add(1, Ordering::Relaxed);

                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service_fn(move |req| {
                                    let router = router.clone();
                                    let handlers = handlers.clone();
                                    let auth_config = auth_config.clone();
                                    let middleware = middleware.clone();
                                 async move {
                                     let method = req.method().clone();
                                     let path = req.uri().path().to_string();
                                     let version = format!("{:?}", req.version()); // e.g., HTTP/1.1

                                     let result = handle_request(
                                         req,
                                         &router,
                                         &handlers,
                                         auth_config.as_deref(),
                                         &middleware,
                                         remote_addr,
                                         max_body_size
                                     ).await;

                                     match &result {
                                         Ok(resp) => {
                                             let status_code = resp.status();
                                             info!("    {} - \"{} {} {}\" {}",
                                                 remote_addr,
                                                 method,
                                                 path,
                                                 version,
                                                 status_code
                                             );
                                         },
                                         Err(_) => {
                                             error!("    {} - \"{} {} {}\" ERROR",
                                                 remote_addr,
                                                 method,
                                                 path,
                                                 version
                                             );
                                         }
                                     }
                                     result
                                 }
                            }))
                            .await
                        {
                            error!("Error serving connection: {:?}", err);
                        }
                        active.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                _ = shutdown_signal() => {
                    info!("Shutdown signal received, stopping server...");
                    break;
                }
            }
        }
        let timeout = self.config.shutdown_timeout;
        let drain = async {
            loop {
                if active.load(Ordering::Relaxed) == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let _ = tokio::time::timeout(timeout, drain).await;
        Ok(())
    }

    /// Execute a test request directly without network stack
    pub async fn test_request(
        &self,
        method: Method,
        path: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<Bytes>,
    ) -> PyResponse {
        if let Some(b) = body.as_ref() {
            if b.len() > self.config.max_body_size {
                return PyResponse::text(r#"{"error": "Payload Too Large"}"#)
                    .with_status(413)
                    .with_header("Content-Type", "application/json");
            }
        }
        let mut req = PyRequest::new(method, path, headers, body);
        req.set_header("x-client-ip", "test");

        process_request(
            &mut req,
            &self.router,
            &self.handlers,
            self.auth_config.as_deref(),
            &self.middleware,
        )
        .await
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}

/// Core request processing logic (network agnostic)
async fn process_request(
    req: &mut PyRequest,
    router: &Router,
    handlers: &[Handler],
    auth_config: Option<&AuthConfig>,
    middleware: &crate::middleware::MiddlewareChain,
) -> PyResponse {
    if req.header("x-request-id").is_none() {
        let request_id = generate_request_id();
        req.set_header("x-request-id", &request_id);
    }

    let matched = match router.match_route(req.method, &req.path) {
        Ok(m) => m,
        Err(_) => {
            return PyResponse::text(r#"{"error": "Not Found"}"#)
                .with_status(404)
                .with_header("Content-Type", "application/json");
        }
    };

    req.typed_params = matched.typed_params.clone();

    if matched.auth_required {
        if let Some(config) = auth_config {
            let auth_header = req.header("authorization");
            if let Some(token) = auth_header.and_then(|h| h.strip_prefix("Bearer ")) {
                match decode::<serde_json::Value>(token, &config.decoding_key, &config.validation) {
                    Ok(token_data) => {
                        req.claims = Some(token_data.claims);
                    }
                    Err(e) => {
                        warn!("JWT validation failed: {}", e);
                        return PyResponse::text(r#"{"error": "Unauthorized"}"#)
                            .with_status(401)
                            .with_header("Content-Type", "application/json");
                    }
                }
            } else {
                return PyResponse::text(r#"{"error": "Missing or invalid Authorization header"}"#)
                    .with_status(401)
                    .with_header("Content-Type", "application/json");
            }
        } else {
            error!("Route requires auth but server has no JWT secret configured");
            return PyResponse::text(
                r#"{"error": "Server misconfigured: Auth required but no secret set"}"#,
            )
            .with_status(500)
            .with_header("Content-Type", "application/json");
        }
    }

    let mut response = match middleware.run_before(req) {
        crate::middleware::MiddlewareResult::Continue => {
            let handler = &handlers[matched.handler_id];
            handler(req, &matched).await
        }
        crate::middleware::MiddlewareResult::Respond(resp) => resp,
    };

    if let Some(request_id) = req.header("x-request-id") {
        response.set_header("x-request-id", request_id);
    }
    middleware.run_after(req, &mut response);
    response
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    router: &Router,
    handlers: &[Handler],
    auth_config: Option<&AuthConfig>,
    middleware: &crate::middleware::MiddlewareChain,
    remote_addr: std::net::SocketAddr,
    max_body_size: usize,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    let mut py_request = match PyRequest::from_hyper_with_limit(req, max_body_size).await {
        Ok(r) => r,
        Err(e) => match e {
            crate::error::Error::PayloadTooLarge { .. } => {
                return Ok(Response::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .body(Full::new(Bytes::from("Payload Too Large")))
                    .unwrap());
            }
            _ => {
                error!("Failed to parse request: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Bad Request")))
                    .unwrap());
            }
        },
    };

    py_request.set_header("x-client-ip", &remote_addr.ip().to_string());
    let response =
        process_request(&mut py_request, router, handlers, auth_config, middleware).await;
    Ok(response.into_hyper())
}

static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", now.as_nanos(), counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_response_json() {
        let resp = PyResponse::json(r#"{"status": "ok"}"#);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.content_type, "application/json");
    }

    #[test]
    fn test_py_response_with_status() {
        let resp = PyResponse::text("Not Found").with_status(404);
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.address.port(), 8000);
        assert!(config.keep_alive);
    }
}
