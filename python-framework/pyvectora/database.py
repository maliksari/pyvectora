"""
PyVectora Database Module

Provides a high-level Python interface to the Rust SQLx database layer.

Design Principles:
- S: Single module for database access
- O: Extensible for different database backends
- D: Abstraction over native bindings

Usage:
    from pyvectora.database import Database

    db = await Database.connect_sqlite("sqlite::memory:")
    await db.execute("CREATE TABLE users (id INTEGER, name TEXT)")
    rows = await db.fetch_all("SELECT * FROM users")
"""

from typing import List, Dict, Any, Optional
from dataclasses import dataclass
from dataclasses import dataclass
try:
    from . import pyvectora_native
except Exception:
    pyvectora_native = None

def _require_native():
    if pyvectora_native is None:
        raise RuntimeError("Native module not available. Run 'maturin develop' to build.")

class Database:
    """
    High-performance async database with connection pooling.

    Powered by Rust SQLx - supports SQLite and PostgreSQL.

    Example:
        >>> db = await Database.connect_sqlite("sqlite::memory:")
        >>> await db.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        >>> await db.execute("INSERT INTO users VALUES (1, 'Alice')")
        >>> rows = await db.fetch_all("SELECT * FROM users")
        >>> print(rows[0])  # {'id': 1, 'name': 'Alice'}
    """

    def __init__(self, native_db: "pyvectora_native.DatabaseNative"):
        """Initialize with native database instance."""
        _require_native()
        self._db = native_db

    @classmethod
    async def connect_sqlite(cls, url: str) -> "Database":
        """
        Connect to a SQLite database.

        Args:
            url: SQLite URL (e.g., "sqlite:mydb.db" or "sqlite::memory:")

        Returns:
            Database instance with connection pool
        """
        if not url.startswith("sqlite:"):
            url = f"sqlite:{url}"

        _require_native()
        native = pyvectora_native.DatabaseNative.connect_sqlite(url)
        return cls(native)

    @classmethod
    async def connect_postgres(cls, url: str) -> "Database":
        """
        Connect to a PostgreSQL database.

        Args:
            url: PostgreSQL URL (e.g., "postgres://user:pass@localhost/db")

        Returns:
            Database instance with connection pool
        """
        _require_native()
        native = pyvectora_native.DatabaseNative.connect_postgres(url)
        return cls(native)

    async def execute(self, query: str) -> int:
        """
        Execute a query that doesn't return rows.

        Args:
            query: SQL query (INSERT, UPDATE, DELETE, CREATE, etc.)

        Returns:
            Number of affected rows
        """
        return await self._db.execute(query)

    async def fetch_all(self, query: str) -> List[Dict[str, Any]]:
        """
        Fetch all rows from a query.

        Args:
            query: SQL SELECT query

        Returns:
            List of row dictionaries
        """
        return await self._db.fetch_all(query)

    async def fetch_one(self, query: str) -> Dict[str, Any]:
        """
        Fetch a single row from a query.

        Args:
            query: SQL SELECT query

        Returns:
            Row as dictionary

        Raises:
            DatabaseError: If no row found
        """
        return await self._db.fetch_one(query)

    async def fetch_optional(self, query: str) -> Optional[Dict[str, Any]]:
        """
        Fetch a single row, returning None if not found.

        Args:
            query: SQL SELECT query

        Returns:
            Row as dictionary, or None
        """
        return await self._db.fetch_optional(query)

    def close(self) -> None:
        """Close the database connection pool."""
        self._db.close()

    @property
    def is_connected(self) -> bool:
        """Check if the pool is connected."""
        return self._db.is_connected

    def transaction(self) -> "Transaction":
        """
        Start a database transaction.
        Returns:
            Transaction context manager
        """
        return Transaction(self)

    async def __aenter__(self) -> "Database":
        """Async Context manager entry."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async Context manager exit - close connection."""
        self.close()

class Transaction:
    """
    Database transaction helper for atomic query batching.
    Supports async context manager.
    """

    def __init__(self, db: Database):
        """Initialize a transaction."""
        self._db = db
        self._queries: List[str] = []
        self._committed = False
        self._entered = False

    def execute(self, query: str) -> None:
        """
        Queue a query for execution.
        """
        if self._committed:
            raise DatabaseError("Transaction already committed")
        self._queries.append(query)

    async def commit(self) -> None:
        """Execute all queued queries."""
        if self._committed:
            return

        for query in self._queries:
            await self._db.execute(query)

        self._committed = True
        self._queries.clear()

    def rollback(self) -> None:
        """Discard all queued queries without executing."""
        self._queries.clear()
        self._committed = True

    async def __aenter__(self) -> "Transaction":
        """Start collecting queries."""
        self._entered = True
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> bool:
        """Execute or discard queries based on success/failure."""
        if exc_type is not None:
            self.rollback()
            return False

        if not self._committed:
            await self.commit()

        return False

DatabaseError = pyvectora_native.DatabaseError if pyvectora_native else RuntimeError

__all__ = [
    "Database",
    "Transaction",
    "DatabaseError",
]
