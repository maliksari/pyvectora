//! # Application State
//!
//! Thread-safe state management for application-wide resources.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Only handles state storage and retrieval
//! - **O**: Extensible via `get::<T>()` for any type
//! - **D**: Handlers depend on abstract state interface

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe application state container
///
/// Stores arbitrary typed values that can be shared across handlers.
/// Uses `Arc<RwLock<...>>` for thread-safe access.
///
/// # Example (Rust side)
///
/// ```ignore
/// let state = AppState::new();
/// state.set("database".to_string(), connection);
/// let conn = state.get::<DatabaseConnection>("database");
/// ```
#[derive(Clone, Default)]
pub struct AppState {
    /// Type-erased storage for named values
    data: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl AppState {
    /// Create a new empty state container
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a value with a string key
    ///
    /// Overwrites any existing value with the same key.
    pub fn set<T: Send + Sync + 'static>(&self, key: impl Into<String>, value: T) {
        let mut data = self.data.write().expect("State lock poisoned");
        data.insert(key.into(), Box::new(value));
    }

    /// Get a cloned value by key
    ///
    /// Returns `None` if key doesn't exist or type doesn't match.
    #[must_use]
    pub fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        let data = self.data.read().expect("State lock poisoned");
        data.get(key)
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned()
    }

    /// Check if a key exists
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        let data = self.data.read().expect("State lock poisoned");
        data.contains_key(key)
    }

    /// Remove a value by key
    pub fn remove(&self, key: &str) -> bool {
        let mut data = self.data.write().expect("State lock poisoned");
        data.remove(key).is_some()
    }

    /// Get the number of stored items
    #[must_use]
    pub fn len(&self) -> usize {
        let data = self.data.read().expect("State lock poisoned");
        data.len()
    }

    /// Check if state is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.data.read().expect("State lock poisoned");
        f.debug_struct("AppState")
            .field("keys", &data.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Type-based state storage (alternative to string keys)
///
/// Uses TypeId for O(1) lookups without string allocation.
#[derive(Clone, Default)]
pub struct TypeState {
    data: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl TypeState {
    /// Create a new empty type state
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a value by its type
    pub fn set<T: Send + Sync + 'static>(&self, value: T) {
        let mut data = self.data.write().expect("TypeState lock poisoned");
        data.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a cloned value by type
    #[must_use]
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let data = self.data.read().expect("TypeState lock poisoned");
        data.get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned()
    }

    /// Check if a type exists
    #[must_use]
    pub fn contains<T: 'static>(&self) -> bool {
        let data = self.data.read().expect("TypeState lock poisoned");
        data.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_set_get() {
        let state = AppState::new();
        state.set("count", 42i32);
        state.set("name", "test".to_string());

        assert_eq!(state.get::<i32>("count"), Some(42));
        assert_eq!(state.get::<String>("name"), Some("test".to_string()));
    }

    #[test]
    fn test_app_state_type_mismatch() {
        let state = AppState::new();
        state.set("count", 42i32);

        assert_eq!(state.get::<String>("count"), None);
    }

    #[test]
    fn test_app_state_missing_key() {
        let state = AppState::new();
        assert_eq!(state.get::<i32>("missing"), None);
    }

    #[test]
    fn test_app_state_overwrite() {
        let state = AppState::new();
        state.set("value", 1i32);
        state.set("value", 2i32);

        assert_eq!(state.get::<i32>("value"), Some(2));
    }

    #[test]
    fn test_app_state_remove() {
        let state = AppState::new();
        state.set("key", "value".to_string());
        assert!(state.contains("key"));

        state.remove("key");
        assert!(!state.contains("key"));
    }

    #[test]
    fn test_app_state_len() {
        let state = AppState::new();
        assert!(state.is_empty());

        state.set("a", 1i32);
        state.set("b", 2i32);
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_type_state_set_get() {
        let state = TypeState::new();
        state.set(42i32);
        state.set("hello".to_string());

        assert_eq!(state.get::<i32>(), Some(42));
        assert_eq!(state.get::<String>(), Some("hello".to_string()));
    }

    #[test]
    fn test_type_state_overwrite() {
        let state = TypeState::new();
        state.set(1i32);
        state.set(2i32);

        assert_eq!(state.get::<i32>(), Some(2));
    }

    #[test]
    fn test_app_state_thread_safe() {
        use std::thread;

        let state = AppState::new();
        let state_clone = state.clone();

        let handle = thread::spawn(move || {
            state_clone.set("thread", 123i32);
        });

        handle.join().unwrap();
        assert_eq!(state.get::<i32>("thread"), Some(123));
    }
}
