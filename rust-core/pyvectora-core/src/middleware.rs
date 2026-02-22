//! # Middleware System
//!
//! Request/response interception for logging, timing, CORS, etc.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Each middleware has a single responsibility
//! - **O**: Extensible via Middleware trait
//! - **D**: Server depends on abstract trait, not concrete implementations

use crate::server::{PyRequest, PyResponse};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, info};

/// Middleware trait for request/response interception
///
/// Middlewares are called in order before the handler, and in reverse order after.
pub trait Middleware: Send + Sync {
    /// Called before the request handler
    ///
    /// Can modify the request or return early with a response.
    fn before_request(&self, _req: &PyRequest) -> MiddlewareResult {
        MiddlewareResult::Continue
    }

    /// Called after the request handler
    ///
    /// Can modify the response or perform logging.
    fn after_response(&self, _req: &PyRequest, _res: &mut PyResponse) {}

    /// Middleware name for logging
    fn name(&self) -> &'static str {
        "Unknown"
    }
}

/// Result of middleware execution
#[derive(Debug)]
pub enum MiddlewareResult {
    /// Continue to next middleware/handler
    Continue,
    /// Short-circuit with this response (skip handler)
    Respond(PyResponse),
}

/// Middleware chain for processing requests
#[derive(Default, Clone)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a middleware to the chain
    pub fn add<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// Execute before_request for all middlewares
    pub fn run_before(&self, req: &PyRequest) -> MiddlewareResult {
        for mw in &self.middlewares {
            match mw.before_request(req) {
                MiddlewareResult::Continue => continue,
                result => return result,
            }
        }
        MiddlewareResult::Continue
    }

    /// Execute after_response for all middlewares (in reverse order)
    pub fn run_after(&self, req: &PyRequest, res: &mut PyResponse) {
        for mw in self.middlewares.iter().rev() {
            mw.after_response(req, res);
        }
    }

    /// Get the number of middlewares
    #[must_use]
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

/// Logging middleware - logs requests in structured JSON format
#[derive(Default)]
pub struct LoggingMiddleware {
    log_headers: bool,
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable header logging
    #[must_use]
    pub fn with_headers(mut self) -> Self {
        self.log_headers = true;
        self
    }
}

impl Middleware for LoggingMiddleware {
    fn before_request(&self, req: &PyRequest) -> MiddlewareResult {
        let request_id = req.header("x-request-id").unwrap_or("-");
        info!(
            method = %req.method,
            path = %req.path,
            request_id = %request_id,
            "Request received"
        );
        MiddlewareResult::Continue
    }

    fn after_response(&self, req: &PyRequest, res: &mut PyResponse) {
        let request_id = req.header("x-request-id").unwrap_or("-");
        info!(
            method = %req.method,
            path = %req.path,
            status = res.status,
            request_id = %request_id,
            "Response sent"
        );
    }

    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

/// Timing middleware - measures request duration
pub struct TimingMiddleware {
    /// Store request start time (thread-local)
    start_times: std::sync::Mutex<std::collections::HashMap<String, Instant>>,
}

impl Default for TimingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl TimingMiddleware {
    /// Create a new timing middleware
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_times: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Middleware for TimingMiddleware {
    fn before_request(&self, req: &PyRequest) -> MiddlewareResult {
        let key = format!("{}:{}", req.method, req.path);
        if let Ok(mut times) = self.start_times.lock() {
            times.insert(key, Instant::now());
        }
        MiddlewareResult::Continue
    }

    fn after_response(&self, req: &PyRequest, _res: &mut PyResponse) {
        let key = format!("{}:{}", req.method, req.path);
        if let Ok(mut times) = self.start_times.lock() {
            if let Some(start) = times.remove(&key) {
                let duration = start.elapsed();
                debug!(
                    method = %req.method,
                    path = %req.path,
                    duration_ms = %duration.as_millis(),
                    "Request timing"
                );
            }
        }
    }

    fn name(&self) -> &'static str {
        "TimingMiddleware"
    }
}

/// CORS middleware - adds Cross-Origin Resource Sharing headers
#[derive(Clone)]
pub struct CorsMiddleware {
    allow_origin: String,
    allow_methods: String,
    allow_headers: String,
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self {
            allow_origin: "*".to_string(),
            allow_methods: "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string(),
            allow_headers: "Content-Type, Authorization".to_string(),
        }
    }
}

impl CorsMiddleware {
    /// Create a new CORS middleware with default settings
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allowed origin
    #[must_use]
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allow_origin = origin.into();
        self
    }

    /// Set allowed methods
    #[must_use]
    pub fn allow_methods(mut self, methods: impl Into<String>) -> Self {
        self.allow_methods = methods.into();
        self
    }

    /// Set allowed headers
    #[must_use]
    pub fn allow_headers(mut self, headers: impl Into<String>) -> Self {
        self.allow_headers = headers.into();
        self
    }

    /// Get the Access-Control-Allow-Origin header value
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.allow_origin
    }
}

impl Middleware for CorsMiddleware {
    fn after_response(&self, _req: &PyRequest, res: &mut PyResponse) {
        res.set_header("Access-Control-Allow-Origin", &self.allow_origin);
        res.set_header("Access-Control-Allow-Methods", &self.allow_methods);
        res.set_header("Access-Control-Allow-Headers", &self.allow_headers);
    }

    fn name(&self) -> &'static str {
        "CorsMiddleware"
    }
}

/// Token bucket rate limiting middleware
pub struct RateLimitMiddleware {
    /// Maximum burst capacity
    capacity: u64,
    /// Tokens refilled per second
    refill_per_sec: u64,
    /// Per-key buckets
    state: Mutex<HashMap<String, Bucket>>,
}

/// Internal token bucket state
struct Bucket {
    tokens: u64,
    last_refill: Instant,
}

impl RateLimitMiddleware {
    /// Create a new rate limiter
    #[must_use]
    pub fn new(capacity: u64, refill_per_sec: u64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            state: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: &str) -> bool {
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let bucket = map.entry(key.to_string()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });
        let elapsed = now.duration_since(bucket.last_refill);
        let refill = (elapsed.as_secs_f64() * self.refill_per_sec as f64) as u64;
        if refill > 0 {
            bucket.tokens = (bucket.tokens + refill).min(self.capacity);
            bucket.last_refill = now;
        }
        if bucket.tokens == 0 {
            return false;
        }
        bucket.tokens -= 1;
        true
    }
}

impl Middleware for RateLimitMiddleware {
    fn before_request(&self, req: &PyRequest) -> MiddlewareResult {
        let key = req.header("x-client-ip").unwrap_or("unknown");
        if self.allow(key) {
            MiddlewareResult::Continue
        } else {
            MiddlewareResult::Respond(
                PyResponse::text(r#"{"error":"Rate limit exceeded"}"#)
                    .with_status(429)
                    .with_header("Content-Type", "application/json"),
            )
        }
    }

    fn name(&self) -> &'static str {
        "RateLimitMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::Method;

    fn create_test_request() -> PyRequest {
        unimplemented!("PyRequest creation requires async context")
    }

    #[test]
    fn test_middleware_chain_empty() {
        let chain = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_middleware_chain_add() {
        let mut chain = MiddlewareChain::new();
        chain.add(LoggingMiddleware::new());
        chain.add(TimingMiddleware::new());

        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_logging_middleware_name() {
        let mw = LoggingMiddleware::new();
        assert_eq!(mw.name(), "LoggingMiddleware");
    }

    #[test]
    fn test_timing_middleware_name() {
        let mw = TimingMiddleware::new();
        assert_eq!(mw.name(), "TimingMiddleware");
    }

    #[test]
    fn test_cors_middleware_default() {
        let mw = CorsMiddleware::new();
        assert_eq!(mw.origin(), "*");
    }

    #[test]
    fn test_cors_middleware_custom_origin() {
        let mw = CorsMiddleware::new().allow_origin("https://example.com");
        assert_eq!(mw.origin(), "https://example.com");
    }
}
