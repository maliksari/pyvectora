"""
PyVectora Repository Pattern

Provides a base class for data access layer with CRUD operations.

Design Principles:
- S: Single responsibility - data access only
- O: Open for extension via subclassing
- D: Depends on Database abstraction

Usage:
    from pyvectora import Repository, Database

    class UserRepository(Repository):
        table = "users"

        def find_by_email(self, email: str):
            return self.fetch_optional(
                f"SELECT * FROM {self.table} WHERE email = {self._escape(email)}"
            )

    db = Database.connect_sqlite("sqlite:db.db")
    users = UserRepository(db)
    all_users = users.find_all()
"""

from typing import Any, Dict, List, Optional, Type, TypeVar, Generic
from abc import ABC

T = TypeVar('T')

class Repository(ABC, Generic[T]):
    """
    Base repository class for database operations.

    Provides CRUD operations and query helpers with SQL injection protection.

    Attributes:
        table: Database table name (must be set by subclass)
        id_column: Primary key column name (default: "id")

    Example:
        class UserRepository(Repository):
            table = "users"

            def find_active(self):
                return self.fetch_all(
                    f"SELECT * FROM {self.table} WHERE is_active = 1"
                )
    """

    table: str = None
    id_column: str = "id"

    def __init__(self, db: "Database"):
        """
        Initialize repository with database connection.

        Args:
            db: Database instance
        """
        if self.table is None:
            raise ValueError(
                f"{self.__class__.__name__} must define 'table' attribute"
            )
        self._db = db

    @property
    def db(self) -> "Database":
        """Get the database connection."""
        return self._db

    async def find_all(self) -> List[Dict[str, Any]]:
        """
        Fetch all records from the table.

        Returns:
            List of row dictionaries
        """
        return await self._db.fetch_all(f"SELECT * FROM {self.table}")

    async def find_by_id(self, id: Any) -> Optional[Dict[str, Any]]:
        """
        Find a record by its primary key.

        Args:
            id: Primary key value

        Returns:
            Row dictionary or None if not found
        """
        query = f"SELECT * FROM {self.table} WHERE {self.id_column} = {self._escape(id)}"
        return await self._db.fetch_optional(query)

    async def create(self, data: Dict[str, Any]) -> int:
        """
        Insert a new record.

        Args:
            data: Dictionary of column -> value pairs

        Returns:
            Number of affected rows (typically 1)
        """
        if not data:
            raise ValueError("Cannot create record with empty data")

        columns = ", ".join(data.keys())
        values = ", ".join(self._escape(v) for v in data.values())
        query = f"INSERT INTO {self.table} ({columns}) VALUES ({values})"

        return await self._db.execute(query)

    async def update(self, id: Any, data: Dict[str, Any]) -> int:
        """
        Update a record by its primary key.

        Args:
            id: Primary key value
            data: Dictionary of column -> new value pairs

        Returns:
            Number of affected rows
        """
        if not data:
            raise ValueError("Cannot update record with empty data")

        set_clause = ", ".join(
            f"{col} = {self._escape(val)}"
            for col, val in data.items()
        )
        query = f"UPDATE {self.table} SET {set_clause} WHERE {self.id_column} = {self._escape(id)}"

        return await self._db.execute(query)

    async def delete(self, id: Any) -> int:
        """
        Delete a record by its primary key.

        Args:
            id: Primary key value

        Returns:
            Number of affected rows
        """
        query = f"DELETE FROM {self.table} WHERE {self.id_column} = {self._escape(id)}"
        return await self._db.execute(query)

    async def count(self) -> int:
        """
        Count all records in the table.

        Returns:
            Number of records
        """
        rows = await self._db.fetch_all(f"SELECT {self.id_column} FROM {self.table}")
        return len(rows)

    async def exists(self, id: Any) -> bool:
        """
        Check if a record exists.

        Args:
            id: Primary key value

        Returns:
            True if record exists
        """
        query = f"SELECT 1 FROM {self.table} WHERE {self.id_column} = {self._escape(id)} LIMIT 1"
        return await self._db.fetch_optional(query) is not None

    async def execute(self, query: str) -> int:
        """Execute a raw query that doesn't return rows."""
        return await self._db.execute(query)

    async def fetch_all(self, query: str) -> List[Dict[str, Any]]:
        """Fetch all rows from a raw query."""
        return await self._db.fetch_all(query)

    async def fetch_one(self, query: str) -> Dict[str, Any]:
        """Fetch a single row from a raw query."""
        return await self._db.fetch_one(query)

    async def fetch_optional(self, query: str) -> Optional[Dict[str, Any]]:
        """Fetch a single row or None from a raw query."""
        return await self._db.fetch_optional(query)

    def _escape(self, value: Any) -> str:
        """
        Escape a value for safe SQL insertion.

        WARNING: This provides basic protection. For production,
        use parameterized queries when available.

        Args:
            value: Value to escape

        Returns:
            SQL-safe string representation
        """
        if value is None:
            return "NULL"

        if isinstance(value, bool):
            return "1" if value else "0"

        if isinstance(value, (int, float)):
            return str(value)

        escaped = str(value).replace("'", "''")
        return f"'{escaped}'"

    def _build_where(self, conditions: Dict[str, Any]) -> str:
        """
        Build a WHERE clause from conditions.

        Args:
            conditions: Dictionary of column -> value pairs

        Returns:
            WHERE clause string (without "WHERE" prefix)
        """
        if not conditions:
            return "1=1"

        clauses = []
        for col, val in conditions.items():
            if val is None:
                clauses.append(f"{col} IS NULL")
            else:
                clauses.append(f"{col} = {self._escape(val)}")

        return " AND ".join(clauses)

    async def find_where(self, **conditions) -> List[Dict[str, Any]]:
        """
        Find records matching conditions.

        Args:
            **conditions: Column -> value pairs

        Returns:
            List of matching rows
        """
        where = self._build_where(conditions)
        return await self._db.fetch_all(f"SELECT * FROM {self.table} WHERE {where}")

    async def find_one_where(self, **conditions) -> Optional[Dict[str, Any]]:
        """
        Find a single record matching conditions.

        Args:
            **conditions: Column -> value pairs

        Returns:
            Matching row or None
        """
        where = self._build_where(conditions)
        return await self._db.fetch_optional(f"SELECT * FROM {self.table} WHERE {where}")

__all__ = ["Repository"]
