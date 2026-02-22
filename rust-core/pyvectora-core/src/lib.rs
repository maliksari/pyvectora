//! # PyVectora Core
//!
//! Core runtime library for the PyVectora framework.
//! Provides high-performance HTTP server, routing, and async bridge functionality.
//!
//! ## Architecture
//!
//! This crate implements the "Inversion of Control" pattern where Rust's Tokio runtime
//! is the main process, and Python code runs as a "managed guest".
//!
//! ## Modules
//!
//! - `server` - HTTP server built on Hyper
//! - `router` - High-performance routing using matchit (radix trie)
//! - `route` - Route metadata and information
//! - `request` - HTTP request wrapper with headers and query parsing
//! - `middleware` - Request/response middleware system
//! - `json` - High-performance JSON parsing with simd-json
//! - `validation` - Structured validation errors
//! - `state` - Thread-safe application state
//! - `database` - SQLx database connectivity (SQLite, PostgreSQL)
//! - `types` - Path parameter types and conversion
//! - `error` - Error types and handling

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod database;
pub mod error;
pub mod json;
pub mod middleware;
pub mod request;
pub mod route;
pub mod router;
pub mod server;
pub mod state;
pub mod types;
pub mod validation;

pub use database::{DatabasePool, DbValue};
pub use error::{Error, Result};
pub use json::{parse_json, to_json};
pub use middleware::{
    CorsMiddleware, LoggingMiddleware, Middleware, MiddlewareChain, RateLimitMiddleware,
    TimingMiddleware,
};
pub use request::PyRequest;
pub use route::RouteInfo;
pub use router::Router;
pub use server::Server;
pub use state::{AppState, TypeState};
pub use types::{ParamType, ParamValue};
pub use validation::{FieldError, ValidationCode, ValidationErrors, ValidationResult};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.1");
    }
}
