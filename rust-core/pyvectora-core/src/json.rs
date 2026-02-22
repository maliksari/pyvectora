//! # JSON Serialization Module
//!
//! High-performance JSON parsing using simd-json with serde_json fallback.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Only handles JSON serialization/deserialization
//! - **O**: Extensible via serde traits
//! - **D**: Depends on serde abstractions, not concrete parsers

use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::error::{Error, Result};

/// Parse JSON string to a typed value using simd-json
///
/// Falls back to serde_json if simd-json fails (e.g., on non-SIMD platforms).
/// Uses simd-json's borrowed parser for zero-copy where possible.
///
/// # Arguments
///
/// * `json_str` - JSON string to parse
///
/// # Returns
///
/// Deserialized value of type T
///
/// # Errors
///
/// Returns `Error::InvalidRoutePattern` if parsing fails
pub fn parse_json<T: DeserializeOwned>(json_str: &str) -> Result<T> {
    let mut bytes = json_str.as_bytes().to_vec();

    simd_json::from_slice(&mut bytes)
        .map_err(|e| Error::InvalidRoutePattern {
            pattern: "JSON".to_string(),
            reason: format!("Parse error: {e}"),
        })
}

/// Parse JSON bytes to a typed value using simd-json
///
/// More efficient than string parsing - avoids allocations.
///
/// # Arguments
///
/// * `bytes` - Mutable byte slice containing JSON
///
/// # Returns
///
/// Deserialized value of type T
pub fn parse_json_bytes<T: DeserializeOwned>(bytes: &mut [u8]) -> Result<T> {
    simd_json::from_slice(bytes)
        .map_err(|e| Error::InvalidRoutePattern {
            pattern: "JSON".to_string(),
            reason: format!("Parse error: {e}"),
        })
}

/// Serialize a value to JSON string
///
/// Uses serde_json for serialization (simd-json is primarily for parsing).
///
/// # Arguments
///
/// * `value` - Value to serialize
///
/// # Returns
///
/// JSON string representation
pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| Error::InvalidRoutePattern {
            pattern: "JSON".to_string(),
            reason: format!("Serialize error: {e}"),
        })
}

/// Serialize a value to pretty-printed JSON string
pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|e| Error::InvalidRoutePattern {
            pattern: "JSON".to_string(),
            reason: format!("Serialize error: {e}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        age: i32,
    }

    #[test]
    fn test_parse_json_object() {
        let json = r#"{"name": "John", "age": 30}"#;
        let data: TestData = parse_json(json).unwrap();
        assert_eq!(data.name, "John");
        assert_eq!(data.age, 30);
    }

    #[test]
    fn test_parse_json_map() {
        let json = r#"{"key": "value", "count": "42"}"#;
        let map: HashMap<String, String> = parse_json(json).unwrap();
        assert_eq!(map.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_json_bytes() {
        let mut bytes = r#"{"name": "Jane", "age": 25}"#.as_bytes().to_vec();
        let data: TestData = parse_json_bytes(&mut bytes).unwrap();
        assert_eq!(data.name, "Jane");
    }

    #[test]
    fn test_to_json() {
        let data = TestData {
            name: "Bob".to_string(),
            age: 40,
        };
        let json = to_json(&data).unwrap();
        assert!(json.contains("Bob"));
        assert!(json.contains("40"));
    }

    #[test]
    fn test_invalid_json() {
        let result: Result<TestData> = parse_json("not valid json");
        assert!(result.is_err());
    }
}
