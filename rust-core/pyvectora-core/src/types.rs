//! # Type System for Path Parameters
//!
//! SOLID-compliant type conversion system for path parameters.
//!
//! ## Design Principles
//!
//! - **S**: Single responsibility - each converter handles one type
//! - **O**: Open for extension via `ParamConverter` trait
//! - **L**: All converters are substitutable via trait
//! - **I**: Small, focused `ParamConverter` trait
//! - **D**: Router depends on trait, not concrete types

use crate::error::{Error, Result};
use std::fmt;

/// Supported path parameter types
///
/// Used during route registration to specify expected types.
/// Default is `String` for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParamType {
    /// String type (default) - no conversion
    #[default]
    String,
    /// Integer type - parses to i64
    Int,
    /// Float type - parses to f64
    Float,
    /// Boolean type - parses "true"/"false" or "1"/"0"
    Bool,
}

impl ParamType {
    /// Parse type specifier from route pattern (e.g., "int" from "{id:int}")
    #[must_use]
    pub fn from_specifier(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "int" | "integer" | "i64" => Self::Int,
            "float" | "f64" | "number" => Self::Float,
            "bool" | "boolean" => Self::Bool,
            _ => Self::String,
        }
    }

    /// Get the type name for error messages
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
        }
    }
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

/// Converted parameter value
///
/// Holds the actual typed value after conversion.
/// Python bindings will convert these to appropriate Python types.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// String value (no conversion performed)
    String(String),
    /// Integer value (i64)
    Int(i64),
    /// Float value (f64)
    Float(f64),
    /// Boolean value
    Bool(bool),
}

impl ParamValue {
    /// Get the value as a string (for backward compatibility)
    #[must_use]
    pub fn as_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Int(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::Bool(b) => b.to_string(),
        }
    }

    /// Check if value is a string
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Get as i64 if Int variant
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as f64 if Float variant
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as bool if Bool variant
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Convert raw string to typed value based on `ParamType`
///
/// This is the core conversion function following DRY principle.
/// All type conversion logic is centralized here.
///
/// # Errors
///
/// Returns `Error::InvalidRoutePattern` if conversion fails.
pub fn convert_param(raw: &str, param_type: ParamType) -> Result<ParamValue> {
    match param_type {
        ParamType::String => Ok(ParamValue::String(raw.to_string())),
        ParamType::Int => raw.parse::<i64>().map(ParamValue::Int).map_err(|_| {
            Error::InvalidRoutePattern {
                pattern: raw.to_string(),
                reason: format!("Cannot convert '{}' to integer", raw),
            }
        }),
        ParamType::Float => raw.parse::<f64>().map(ParamValue::Float).map_err(|_| {
            Error::InvalidRoutePattern {
                pattern: raw.to_string(),
                reason: format!("Cannot convert '{}' to float", raw),
            }
        }),
        ParamType::Bool => match raw.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(ParamValue::Bool(true)),
            "false" | "0" | "no" => Ok(ParamValue::Bool(false)),
            _ => Err(Error::InvalidRoutePattern {
                pattern: raw.to_string(),
                reason: format!("Cannot convert '{}' to boolean", raw),
            }),
        },
    }
}

/// Parse a path segment pattern to extract name and type
///
/// Examples:
/// - `{id}` -> ("id", ParamType::String)
/// - `{id:int}` -> ("id", ParamType::Int)
/// - `{price:float}` -> ("price", ParamType::Float)
///
/// # Returns
///
/// `Some((name, type))` if pattern is a parameter, `None` if static segment.
#[must_use]
pub fn parse_param_pattern(segment: &str) -> Option<(String, ParamType)> {
    if segment.starts_with('{') && segment.ends_with('}') {
        let inner = &segment[1..segment.len() - 1];

        if let Some((name, type_spec)) = inner.split_once(':') {
            Some((name.to_string(), ParamType::from_specifier(type_spec)))
        } else {
            Some((inner.to_string(), ParamType::String))
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_type_from_specifier() {
        assert_eq!(ParamType::from_specifier("int"), ParamType::Int);
        assert_eq!(ParamType::from_specifier("INT"), ParamType::Int);
        assert_eq!(ParamType::from_specifier("integer"), ParamType::Int);
        assert_eq!(ParamType::from_specifier("float"), ParamType::Float);
        assert_eq!(ParamType::from_specifier("bool"), ParamType::Bool);
        assert_eq!(ParamType::from_specifier("unknown"), ParamType::String);
    }

    #[test]
    fn test_convert_string() {
        let result = convert_param("hello", ParamType::String).unwrap();
        assert_eq!(result, ParamValue::String("hello".to_string()));
    }

    #[test]
    fn test_convert_int() {
        let result = convert_param("123", ParamType::Int).unwrap();
        assert_eq!(result, ParamValue::Int(123));

        let result = convert_param("-456", ParamType::Int).unwrap();
        assert_eq!(result, ParamValue::Int(-456));
    }

    #[test]
    fn test_convert_int_invalid() {
        let result = convert_param("abc", ParamType::Int);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_float() {
        let result = convert_param("3.14", ParamType::Float).unwrap();
        assert_eq!(result, ParamValue::Float(3.14));
    }

    #[test]
    fn test_convert_bool() {
        assert_eq!(convert_param("true", ParamType::Bool).unwrap(), ParamValue::Bool(true));
        assert_eq!(convert_param("false", ParamType::Bool).unwrap(), ParamValue::Bool(false));
        assert_eq!(convert_param("1", ParamType::Bool).unwrap(), ParamValue::Bool(true));
        assert_eq!(convert_param("0", ParamType::Bool).unwrap(), ParamValue::Bool(false));
    }

    #[test]
    fn test_parse_param_pattern() {
        assert_eq!(
            parse_param_pattern("{id}"),
            Some(("id".to_string(), ParamType::String))
        );
        assert_eq!(
            parse_param_pattern("{id:int}"),
            Some(("id".to_string(), ParamType::Int))
        );
        assert_eq!(
            parse_param_pattern("{price:float}"),
            Some(("price".to_string(), ParamType::Float))
        );
        assert_eq!(parse_param_pattern("static"), None);
    }

    #[test]
    fn test_param_value_as_string() {
        assert_eq!(ParamValue::Int(42).as_string(), "42");
        assert_eq!(ParamValue::Float(3.14).as_string(), "3.14");
        assert_eq!(ParamValue::Bool(true).as_string(), "true");
    }
}
