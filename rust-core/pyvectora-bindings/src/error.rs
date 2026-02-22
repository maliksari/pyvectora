//! # PyVectora Error Handling
//!
//! Comprehensive error handling for FFI boundary safety.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Single module for all error types and mapping
//! - **O**: Extensible error variants without breaking changes
//! - **L**: All errors implement common traits (Display, Error)
//! - **D**: Python exceptions abstracted via traits

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::create_exception;
use std::panic::UnwindSafe;

create_exception!(pyvectora, PyVectoraError, pyo3::exceptions::PyException);

create_exception!(pyvectora, ValidationError, PyVectoraError);
create_exception!(pyvectora, NotFoundError, PyVectoraError);
create_exception!(pyvectora, AuthenticationError, PyVectoraError);
create_exception!(pyvectora, DatabaseError, PyVectoraError);
create_exception!(pyvectora, ConfigurationError, PyVectoraError);

/// Internal error type for bindings layer
#[derive(Debug)]
pub enum BindingsError {
    /// Handler panicked during execution
    Panic(String),
    /// Python callback failed
    PythonCallback(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Configuration error
    Configuration(String),
    /// Database error
    Database(String),
    /// Not found error
    NotFound(String),
    /// Validation error
    Validation(String),
    /// Authentication error
    Authentication(String),
}

impl std::fmt::Display for BindingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panic(msg) => write!(f, "Internal panic: {}", msg),
            Self::PythonCallback(msg) => write!(f, "Python callback error: {}", msg),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Self::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
            Self::Authentication(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for BindingsError {}

impl From<BindingsError> for PyErr {
    fn from(err: BindingsError) -> PyErr {
        match err {
            BindingsError::Panic(msg) => PyRuntimeError::new_err(format!("Internal error: {}", msg)),
            BindingsError::PythonCallback(msg) => PyRuntimeError::new_err(msg),
            BindingsError::Serialization(msg) => PyValueError::new_err(msg),
            BindingsError::Configuration(msg) => ConfigurationError::new_err(msg),
            BindingsError::Database(msg) => DatabaseError::new_err(msg),
            BindingsError::NotFound(msg) => NotFoundError::new_err(msg),
            BindingsError::Validation(msg) => ValidationError::new_err(msg),
            BindingsError::Authentication(msg) => AuthenticationError::new_err(msg),
        }
    }
}

/// Result of a panic-safe operation
pub type PanicSafeResult<T> = Result<T, BindingsError>;

/// Execute a closure with panic catching
///
/// Converts any panic into a BindingsError::Panic
///
/// # Example
/// ```ignore
/// let result = catch_panic(|| {
///     some_potentially_panicking_code()
/// });
/// ```
pub fn catch_panic<F, T>(f: F) -> PanicSafeResult<T>
where
    F: FnOnce() -> T + UnwindSafe,
{
    std::panic::catch_unwind(f).map_err(|panic_payload| {
        let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        BindingsError::Panic(msg)
    })
}

/// Execute a closure with panic catching, returning a default on panic
///
/// Useful for FFI boundaries where we must return a value
pub fn catch_panic_with_default<F, T>(f: F, default: T) -> T
where
    F: FnOnce() -> T + UnwindSafe,
{
    std::panic::catch_unwind(f).unwrap_or(default)
}

/// Register error types with Python module
pub fn register_exceptions(m: &PyModule) -> PyResult<()> {
    m.add("PyVectoraError", m.py().get_type::<PyVectoraError>())?;
    m.add("ValidationError", m.py().get_type::<ValidationError>())?;
    m.add("NotFoundError", m.py().get_type::<NotFoundError>())?;
    m.add("AuthenticationError", m.py().get_type::<AuthenticationError>())?;
    m.add("DatabaseError", m.py().get_type::<DatabaseError>())?;
    m.add("ConfigurationError", m.py().get_type::<ConfigurationError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catch_panic_success() {
        let result = catch_panic(|| 42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_catch_panic_failure() {
        let result = catch_panic(|| {
            panic!("Test panic message");
        });
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, BindingsError::Panic(_)));
        assert!(err.to_string().contains("Test panic message"));
    }

    #[test]
    fn test_catch_panic_with_default() {
        let result = catch_panic_with_default(|| panic!("boom"), 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_error_display() {
        let err = BindingsError::NotFound("User 123".to_string());
        assert_eq!(err.to_string(), "Not found: User 123");
    }
}
