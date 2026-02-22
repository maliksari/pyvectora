//! # PyVectora Database Python Bindings
//!
//! Exposes Rust SQLx database layer to Python with async support.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Only handles database Python bindings
//! - **O**: Extensible for new database backends
//! - **L**: All pool types implement common interface
//! - **D**: Python depends on abstractions, not concrete implementations
//!
//! ## Performance Notes
//!
//! - Connection pooling handled by SQLx
//! - GIL released during all I/O operations
//! - Results converted to Python dicts efficiently

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::exceptions::PyRuntimeError;
use pyvectora_core::database::{DatabasePool, DbValue};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::get_runtime;
use crate::error::DatabaseError;

/// Python-exposed database connection pool
///
/// Provides async database operations with connection pooling.
/// Supports SQLite and PostgreSQL backends.
///
/// # Example (Python)
///
/// ```python
/// pool = await Database.connect_sqlite("sqlite::memory:")
/// await pool.execute("CREATE TABLE users (id INTEGER, name TEXT)")
/// rows = await pool.fetch_all("SELECT * FROM users")
/// ```
/// ```
#[pyclass(name = "DatabaseNative")]
pub struct PyDatabaseNative {
    /// Inner pool wrapped in Arc for cloning across async boundaries
    inner: Arc<RwLock<Option<DatabasePool>>>,
}

#[pymethods]
impl PyDatabaseNative {
    /// Connect to a SQLite database
    ///
    /// Args:
    ///     url: Database URL (e.g., "sqlite:mydb.db" or "sqlite::memory:")
    ///     max_connections: Maximum pool size (default: 10)
    ///
    /// Returns:
    ///     Database instance with connection pool
    #[staticmethod]
    #[pyo3(signature = (url, max_connections=None))]
    fn connect_sqlite(py: Python<'_>, url: String, max_connections: Option<u32>) -> PyResult<Self> {
        let pool = py.allow_threads(|| {
            get_runtime().block_on(async {
                DatabasePool::connect_sqlite(&url, max_connections).await
            })
        }).map_err(|e| DatabaseError::new_err(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(RwLock::new(Some(pool))),
        })
    }

    /// Connect to a PostgreSQL database
    ///
    /// Args:
    ///     url: Database URL (e.g., "postgres://user:pass@host/db")
    ///     max_connections: Maximum pool size (default: 10)
    ///
    /// Returns:
    ///     Database instance with connection pool
    #[staticmethod]
    #[pyo3(signature = (url, max_connections=None))]
    fn connect_postgres(py: Python<'_>, url: String, max_connections: Option<u32>) -> PyResult<Self> {
        let pool = py.allow_threads(|| {
            get_runtime().block_on(async {
                DatabasePool::connect_postgres(&url, max_connections).await
            })
        }).map_err(|e| DatabaseError::new_err(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(RwLock::new(Some(pool))),
        })
    }

    /// Execute a query that doesn't return rows (INSERT, UPDATE, DELETE)
    ///
    /// Args:
    ///     query: SQL query string
    ///
    /// Returns:
    ///     Number of affected rows
    #[pyo3(text_signature = "($self, query)")]
    fn execute<'p>(&self, py: Python<'p>, query: String) -> PyResult<&'p PyAny> {
        let inner = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py::<_, u64>(py, async move {
            let guard = inner.read().await;
            let pool = guard.as_ref()
                .ok_or_else(|| PyRuntimeError::new_err("Database pool is closed"))?;

            pool.execute(&query).await
                .map_err(|e| DatabaseError::new_err(e.to_string()))
        })
    }

    /// Fetch all rows from a query
    ///
    /// Args:
    ///     query: SQL query string
    ///
    /// Returns:
    ///     List of dictionaries, one per row
    #[pyo3(text_signature = "($self, query)")]
    fn fetch_all<'p>(&self, py: Python<'p>, query: String) -> PyResult<&'p PyAny> {
        let inner = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let rows = {
                let guard = inner.read().await;
                let pool = guard.as_ref()
                    .ok_or_else(|| PyRuntimeError::new_err("Database pool is closed"))?;

                pool.fetch_all(&query).await
                    .map_err(|e| DatabaseError::new_err(e.to_string()))?
            };

            Python::with_gil(|py| {
                 let list = PyList::empty(py);
                 for row in rows {
                     let dict = convert_row_to_dict(py, row)?;
                     list.append(dict)?;
                 }
                 Ok(list.to_object(py))
            })
        })
    }

    /// Fetch a single row from a query
    ///
    /// Args:
    ///     query: SQL query string
    ///
    /// Returns:
    ///     Dictionary representing the row, or None if not found
    #[pyo3(text_signature = "($self, query)")]
    fn fetch_one<'p>(&self, py: Python<'p>, query: String) -> PyResult<&'p PyAny> {
        let inner = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let row = {
                let guard = inner.read().await;
                let pool = guard.as_ref()
                    .ok_or_else(|| PyRuntimeError::new_err("Database pool is closed"))?;

                pool.fetch_one(&query).await
                    .map_err(|e| DatabaseError::new_err(e.to_string()))?
            };

            Python::with_gil(|py| {
                convert_row_to_dict(py, row).map(|d| d.to_object(py))
            })
        })
    }

    #[pyo3(text_signature = "($self, query)")]
    fn fetch_optional<'p>(&self, py: Python<'p>, query: String) -> PyResult<&'p PyAny> {
        let inner = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let option_row = {
                let guard = inner.read().await;
                match guard.as_ref() {
                     Some(pool) => pool.fetch_optional(&query).await
                         .map_err(|e| DatabaseError::new_err(e.to_string()))?,
                     None => return Err(PyRuntimeError::new_err("Database pool is closed")),
                }
            };

            Python::with_gil(|py| {
                match option_row {
                    Some(row) => convert_row_to_dict(py, row).map(|d| d.to_object(py)),
                    None => Ok(py.None()),
                }
            })
        })
    }

    /// Close the database connection pool
    ///
    /// After closing, all operations will fail.
    #[pyo3(text_signature = "($self)")]
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        let inner = self.inner.clone();

        py.allow_threads(|| {
            get_runtime().block_on(async {
                let mut guard = inner.write().await;
                if let Some(pool) = guard.take() {
                    pool.close().await;
                }
                Ok(())
            })
        })
    }

    /// Check if the pool is connected
    #[getter]
    fn is_connected(&self, py: Python<'_>) -> bool {
        let inner = self.inner.clone();

        py.allow_threads(|| {
            get_runtime().block_on(async {
                let guard = inner.read().await;
                guard.is_some()
            })
        })
    }
}

/// Convert a database row (HashMap<String, DbValue>) to Python dict
fn convert_row_to_dict<'py>(
    py: Python<'py>,
    row: std::collections::HashMap<String, DbValue>
) -> PyResult<&'py PyDict> {
    let dict = PyDict::new(py);

    for (key, value) in row {
        let py_value = convert_db_value(py, &value)?;
        dict.set_item(key, py_value)?;
    }

    Ok(dict)
}

/// Convert DbValue to Python object
fn convert_db_value(py: Python<'_>, value: &DbValue) -> PyResult<PyObject> {
    Ok(match value {
        DbValue::Null => py.None(),
        DbValue::Int(i) => i.to_object(py),
        DbValue::Float(f) => f.to_object(py),
        DbValue::String(s) => s.to_object(py),
        DbValue::Bool(b) => b.to_object(py),
        DbValue::Bytes(bytes) => bytes.to_object(py),
    })
}

/// Register database classes with Python module
pub fn register_database_classes(m: &PyModule) -> PyResult<()> {
    m.add_class::<PyDatabaseNative>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_struct_creation() {
        let pool: Arc<RwLock<Option<DatabasePool>>> = Arc::new(RwLock::new(None));
        assert!(pool.try_read().unwrap().is_none());
    }
}
