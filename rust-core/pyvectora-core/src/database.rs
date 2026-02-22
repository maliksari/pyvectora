//! # Database Module
//!
//! Async database connectivity with SQLx for PostgreSQL and SQLite.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Only handles database operations
//! - **O**: DatabasePool enum extensible for new backends
//! - **D**: Abstraction over specific database drivers

use crate::error::{Error, Result};
use serde::Serialize;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Row, TypeInfo};
use std::collections::HashMap;

/// Database connection pool supporting multiple backends
#[derive(Clone)]
pub enum DatabasePool {
    /// SQLite connection pool
    Sqlite(SqlitePool),
    /// PostgreSQL connection pool
    Postgres(PgPool),
}

impl DatabasePool {
    /// Connect to a SQLite database
    ///
    /// # Arguments
    ///
    /// * `url` - Database URL (e.g., "sqlite:mydb.db" or ":memory:")
    /// * `max_connections` - Maximum pool size (default: 10)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pool = DatabasePool::connect_sqlite("sqlite::memory:", None).await?;
    /// let pool = DatabasePool::connect_sqlite("sqlite:db.db", Some(20)).await?;
    /// ```
    pub async fn connect_sqlite(url: &str, max_connections: Option<u32>) -> Result<Self> {
        let pool_size = max_connections.unwrap_or(10);
        let pool = SqlitePoolOptions::new()
            .max_connections(pool_size)
            .connect(url)
            .await
            .map_err(|e| Error::Database {
                message: format!("SQLite connection failed: {e}"),
            })?;

        Ok(Self::Sqlite(pool))
    }

    /// Connect to a PostgreSQL database
    ///
    /// # Arguments
    ///
    /// * `url` - Database URL (e.g., "postgres://user:pass@host/db")
    /// * `max_connections` - Maximum pool size (default: 10)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pool = DatabasePool::connect_postgres("postgres://localhost/mydb", None).await?;
    /// ```
    pub async fn connect_postgres(url: &str, max_connections: Option<u32>) -> Result<Self> {
        let pool_size = max_connections.unwrap_or(10);
        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
            .connect(url)
            .await
            .map_err(|e| Error::Database {
                message: format!("PostgreSQL connection failed: {e}"),
            })?;

        Ok(Self::Postgres(pool))
    }

    /// Execute a query that doesn't return rows (INSERT, UPDATE, DELETE)
    ///
    /// Returns the number of affected rows.
    pub async fn execute(&self, query: &str) -> Result<u64> {
        match self {
            Self::Sqlite(pool) => {
                let result =
                    sqlx::query(query)
                        .execute(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;
                Ok(result.rows_affected())
            }
            Self::Postgres(pool) => {
                let result =
                    sqlx::query(query)
                        .execute(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;
                Ok(result.rows_affected())
            }
        }
    }

    /// Fetch all rows from a query
    ///
    /// Returns rows as a vector of HashMaps for easy Python conversion.
    pub async fn fetch_all(&self, query: &str) -> Result<Vec<HashMap<String, DbValue>>> {
        match self {
            Self::Sqlite(pool) => {
                let rows: Vec<SqliteRow> =
                    sqlx::query(query)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;

                Ok(rows.iter().map(sqlite_row_to_map).collect())
            }
            Self::Postgres(pool) => {
                let rows: Vec<PgRow> =
                    sqlx::query(query)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;

                Ok(rows.iter().map(pg_row_to_map).collect())
            }
        }
    }

    /// Fetch a single row (optional)
    pub async fn fetch_optional(&self, query: &str) -> Result<Option<HashMap<String, DbValue>>> {
        match self {
            Self::Sqlite(pool) => {
                let row: Option<SqliteRow> = sqlx::query(query)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| Error::Database {
                        message: format!("Query error: {e}"),
                    })?;

                Ok(row.map(|r| sqlite_row_to_map(&r)))
            }
            Self::Postgres(pool) => {
                let row: Option<PgRow> =
                    sqlx::query(query)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;

                Ok(row.map(|r| pg_row_to_map(&r)))
            }
        }
    }

    /// Fetch a single row from a query
    pub async fn fetch_one(&self, query: &str) -> Result<HashMap<String, DbValue>> {
        match self {
            Self::Sqlite(pool) => {
                let row: SqliteRow =
                    sqlx::query(query)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;

                Ok(sqlite_row_to_map(&row))
            }
            Self::Postgres(pool) => {
                let row: PgRow =
                    sqlx::query(query)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| Error::Database {
                            message: format!("Query error: {e}"),
                        })?;

                Ok(pg_row_to_map(&row))
            }
        }
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        match self {
            Self::Sqlite(pool) => pool.close().await,
            Self::Postgres(pool) => pool.close().await,
        }
    }
}

/// Database value types for Python conversion
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DbValue {
    /// Null value
    Null,
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// Binary data
    Bytes(Vec<u8>),
}

/// Convert SQLite row to HashMap
fn sqlite_row_to_map(row: &SqliteRow) -> HashMap<String, DbValue> {
    let mut map = HashMap::new();

    for (i, column) in row.columns().iter().enumerate() {
        let name = column.name().to_string();
        let type_name = column.type_info().name();

        let value = match type_name {
            "INTEGER" => row
                .try_get::<i64, _>(i)
                .map(DbValue::Int)
                .unwrap_or(DbValue::Null),
            "REAL" => row
                .try_get::<f64, _>(i)
                .map(DbValue::Float)
                .unwrap_or(DbValue::Null),
            "TEXT" => row
                .try_get::<String, _>(i)
                .map(DbValue::String)
                .unwrap_or(DbValue::Null),
            "BLOB" => row
                .try_get::<Vec<u8>, _>(i)
                .map(DbValue::Bytes)
                .unwrap_or(DbValue::Null),
            _ => row
                .try_get::<String, _>(i)
                .map(DbValue::String)
                .unwrap_or(DbValue::Null),
        };

        map.insert(name, value);
    }

    map
}

/// Convert PostgreSQL row to HashMap
fn pg_row_to_map(row: &PgRow) -> HashMap<String, DbValue> {
    let mut map = HashMap::new();

    for (i, column) in row.columns().iter().enumerate() {
        let name = column.name().to_string();
        let type_name = column.type_info().name();

        let value = match type_name {
            "INT2" | "INT4" | "INT8" => row
                .try_get::<i64, _>(i)
                .map(DbValue::Int)
                .unwrap_or(DbValue::Null),
            "FLOAT4" | "FLOAT8" => row
                .try_get::<f64, _>(i)
                .map(DbValue::Float)
                .unwrap_or(DbValue::Null),
            "BOOL" => row
                .try_get::<bool, _>(i)
                .map(DbValue::Bool)
                .unwrap_or(DbValue::Null),
            "BYTEA" => row
                .try_get::<Vec<u8>, _>(i)
                .map(DbValue::Bytes)
                .unwrap_or(DbValue::Null),
            _ => row
                .try_get::<String, _>(i)
                .map(DbValue::String)
                .unwrap_or(DbValue::Null),
        };

        map.insert(name, value);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_memory_connection() {
        let pool = DatabasePool::connect_sqlite("sqlite::memory:", None).await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_create_table() {
        let pool = DatabasePool::connect_sqlite("sqlite::memory:", None)
            .await
            .unwrap();

        let result = pool
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sqlite_insert_and_fetch() {
        let pool = DatabasePool::connect_sqlite("sqlite::memory:", None)
            .await
            .unwrap();

        pool.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();
        pool.execute("INSERT INTO users (id, name) VALUES (1, 'Alice')")
            .await
            .unwrap();
        pool.execute("INSERT INTO users (id, name) VALUES (2, 'Bob')")
            .await
            .unwrap();

        let rows = pool.fetch_all("SELECT * FROM users").await.unwrap();

        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_fetch_one() {
        let pool = DatabasePool::connect_sqlite("sqlite::memory:", None)
            .await
            .unwrap();

        pool.execute("CREATE TABLE config (key TEXT, value TEXT)")
            .await
            .unwrap();
        pool.execute("INSERT INTO config VALUES ('debug', 'true')")
            .await
            .unwrap();

        let row = pool
            .fetch_one("SELECT * FROM config WHERE key = 'debug'")
            .await
            .unwrap();

        assert!(row.contains_key("key"));
        assert!(row.contains_key("value"));
    }
}
