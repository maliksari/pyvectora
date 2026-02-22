//! # Error Handling
//!
//! Centralized error types for PyVectora core.
//! Uses `thiserror` for ergonomic error definitions.

use thiserror::Error;

/// Result type alias for PyVectora operations
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for the PyVectora runtime
#[derive(Error, Debug)]
pub enum Error {
    /// Server failed to bind to the specified address
    #[error("Failed to bind server to {address}: {source}")]
    BindError {
        /// The address we tried to bind to
        address: String,
        /// The underlying IO error
        #[source]
        source: std::io::Error,
    },

    /// Router failed to match the requested path
    #[error("No route found for path: {path}")]
    RouteNotFound {
        /// The path that wasn't matched
        path: String,
    },

    /// Invalid route pattern provided
    #[error("Invalid route pattern: {pattern}: {reason}")]
    InvalidRoutePattern {
        /// The invalid pattern
        pattern: String,
        /// Reason for invalidity
        reason: String,
    },

    /// HTTP protocol error
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Python interop error (for FFI bridge)
    #[error("Python error: {message}")]
    Python {
        /// Error message from Python
        message: String,
    },

    /// Database error
    #[error("Database error: {message}")]
    Database {
        /// Error message from database
        message: String,
    },

    /// Request payload too large
    #[error("Payload too large: limit={limit} bytes, received={actual} bytes")]
    PayloadTooLarge {
        /// Max allowed size
        limit: usize,
        /// Actual size
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_not_found_error() {
        let err = Error::RouteNotFound {
            path: "/unknown".to_string(),
        };
        assert!(err.to_string().contains("/unknown"));
    }

    #[test]
    fn test_bind_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "address in use");
        let err = Error::BindError {
            address: "0.0.0.0:8000".to_string(),
            source: io_err,
        };
        assert!(err.to_string().contains("0.0.0.0:8000"));
    }
}
