//! # Validation Module
//!
//! Structured validation errors for API responses.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Only handles validation error representation
//! - **O**: Extensible error codes via enum
//! - **L**: All validation errors implement common traits

use serde::Serialize;
use std::collections::HashMap;

/// Error code for categorizing validation failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationCode {
    /// Required field is missing
    Required,
    /// Value is invalid type
    InvalidType,
    /// Value is too short
    TooShort,
    /// Value is too long
    TooLong,
    /// Value is below minimum
    TooSmall,
    /// Value is above maximum
    TooLarge,
    /// Value doesn't match pattern
    InvalidFormat,
    /// Value is not unique
    NotUnique,
    /// Value is not in allowed set
    InvalidChoice,
    /// Custom validation failed
    Custom,
}

/// A single validation error for a specific field
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    /// Field name (e.g., "email", "user.address.city")
    pub field: String,
    /// Human-readable error message
    pub message: String,
    /// Machine-readable error code
    pub code: ValidationCode,
}

impl FieldError {
    /// Create a new field error
    pub fn new(field: impl Into<String>, message: impl Into<String>, code: ValidationCode) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code,
        }
    }

    /// Create a "required field" error
    pub fn required(field: impl Into<String>) -> Self {
        let field_str = field.into();
        Self {
            message: format!("{} is required", field_str),
            field: field_str,
            code: ValidationCode::Required,
        }
    }

    /// Create an "invalid type" error
    pub fn invalid_type(field: impl Into<String>, expected: &str) -> Self {
        let field_str = field.into();
        Self {
            message: format!("{} must be {}", field_str, expected),
            field: field_str,
            code: ValidationCode::InvalidType,
        }
    }

    /// Create a "too short" error
    pub fn too_short(field: impl Into<String>, min: usize) -> Self {
        let field_str = field.into();
        Self {
            message: format!("{} must be at least {} characters", field_str, min),
            field: field_str,
            code: ValidationCode::TooShort,
        }
    }

    /// Create a "too long" error
    pub fn too_long(field: impl Into<String>, max: usize) -> Self {
        let field_str = field.into();
        Self {
            message: format!("{} must be at most {} characters", field_str, max),
            field: field_str,
            code: ValidationCode::TooLong,
        }
    }
}

/// Collection of validation errors
///
/// Allows aggregating multiple field errors for a single request.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationErrors {
    /// List of field-level errors
    pub errors: Vec<FieldError>,
}

impl ValidationErrors {
    /// Create an empty error collection
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field error
    pub fn add(&mut self, error: FieldError) {
        self.errors.push(error);
    }

    /// Add a required field error
    pub fn add_required(&mut self, field: impl Into<String>) {
        self.add(FieldError::required(field));
    }

    /// Check if there are any errors
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Convert to JSON response body
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"errors":[]}"#.to_string())
    }

    /// Group errors by field
    #[must_use]
    pub fn by_field(&self) -> HashMap<String, Vec<&FieldError>> {
        let mut map: HashMap<String, Vec<&FieldError>> = HashMap::new();
        for error in &self.errors {
            map.entry(error.field.clone()).or_default().push(error);
        }
        map
    }
}

/// Result type for validation operations
pub type ValidationResult<T> = std::result::Result<T, ValidationErrors>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_error_required() {
        let error = FieldError::required("email");
        assert_eq!(error.field, "email");
        assert_eq!(error.code, ValidationCode::Required);
        assert!(error.message.contains("required"));
    }

    #[test]
    fn test_validation_errors_add() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.add_required("email");
        errors.add(FieldError::too_short("password", 8));

        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_validation_errors_json() {
        let mut errors = ValidationErrors::new();
        errors.add_required("email");

        let json = errors.to_json();
        assert!(json.contains("email"));
        assert!(json.contains("REQUIRED"));
    }

    #[test]
    fn test_field_error_helpers() {
        let e1 = FieldError::invalid_type("age", "integer");
        assert_eq!(e1.code, ValidationCode::InvalidType);

        let e2 = FieldError::too_short("name", 3);
        assert_eq!(e2.code, ValidationCode::TooShort);

        let e3 = FieldError::too_long("bio", 500);
        assert_eq!(e3.code, ValidationCode::TooLong);
    }

    #[test]
    fn test_by_field() {
        let mut errors = ValidationErrors::new();
        errors.add(FieldError::required("email"));
        errors.add(FieldError::invalid_type("email", "string"));
        errors.add(FieldError::required("name"));

        let grouped = errors.by_field();
        assert_eq!(grouped.get("email").map(|v| v.len()), Some(2));
        assert_eq!(grouped.get("name").map(|v| v.len()), Some(1));
    }
}
