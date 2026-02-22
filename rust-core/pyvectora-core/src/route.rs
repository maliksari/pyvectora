//! # Route Metadata
//!
//! Single-responsibility module for route information.
//!
//! ## Design Principles
//!
//! - **S**: RouteInfo only holds route metadata
//! - **O**: Extensible via additional fields without breaking changes
//! - **D**: Decoupled from Router implementation details

use crate::router::HandlerId;
use crate::types::ParamType;
use std::collections::HashMap;

/// Route metadata containing handler and type information
///
/// This struct follows Single Responsibility Principle -
/// it only holds information about a single route definition.
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// Unique handler identifier
    pub handler_id: HandlerId,
    /// Original path pattern (e.g., "/users/{id:int}")
    pub path_pattern: String,
    /// Normalized path for matchit (e.g., "/users/{id}")
    pub match_pattern: String,
    /// Parameter name to type mapping
    pub param_types: HashMap<String, ParamType>,
    /// Whether authentication is required for this route
    pub auth_required: bool,
}

impl RouteInfo {
    /// Create a new RouteInfo from a path pattern
    ///
    /// Parses the pattern to extract parameter types and creates
    /// a normalized pattern for matchit routing.
    ///
    /// # Arguments
    ///
    /// * `handler_id` - The assigned handler ID
    /// * `path` - Path pattern with optional type specifiers (e.g., "/users/{id:int}")
    /// * `auth_required` - Whether to enforce JWT validation
    #[must_use]
    pub fn new(handler_id: HandlerId, path: &str, auth_required: bool) -> Self {
        let (match_pattern, param_types) = Self::parse_path_pattern(path);

        Self {
            handler_id,
            path_pattern: path.to_string(),
            match_pattern,
            param_types,
            auth_required,
        }
    }

    /// Parse path pattern to extract parameter types
    ///
    /// Converts `{name:type}` to `{name}` for matchit compatibility
    /// and builds the param_types map.
    ///
    /// # Returns
    ///
    /// Tuple of (normalized_pattern, param_types_map)
    fn parse_path_pattern(path: &str) -> (String, HashMap<String, ParamType>) {
        let mut param_types = HashMap::new();
        let mut normalized_parts = Vec::new();

        for segment in path.split('/') {
            if segment.is_empty() {
                continue;
            }

            if let Some((name, param_type)) = crate::types::parse_param_pattern(segment) {
                param_types.insert(name.clone(), param_type);
                normalized_parts.push(format!("{{{}}}", name));
            } else {
                normalized_parts.push(segment.to_string());
            }
        }

        let normalized = if normalized_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", normalized_parts.join("/"))
        };

        (normalized, param_types)
    }

    /// Get the type for a parameter by name
    ///
    /// Returns `ParamType::String` if parameter not found (backward compatible)
    #[must_use]
    pub fn get_param_type(&self, name: &str) -> ParamType {
        self.param_types.get(name).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_info_simple() {
        let info = RouteInfo::new(0, "/users", false);
        assert_eq!(info.match_pattern, "/users");
        assert!(info.param_types.is_empty());
        assert!(!info.auth_required);
    }

    #[test]
    fn test_route_info_with_string_param() {
        let info = RouteInfo::new(0, "/users/{id}", true);
        assert_eq!(info.match_pattern, "/users/{id}");
        assert_eq!(info.get_param_type("id"), ParamType::String);
        assert!(info.auth_required);
    }

    #[test]
    fn test_route_info_with_typed_param() {
        let info = RouteInfo::new(0, "/users/{id:int}", false);
        assert_eq!(info.match_pattern, "/users/{id}");
        assert_eq!(info.get_param_type("id"), ParamType::Int);
    }

    #[test]
    fn test_route_info_multiple_params() {
        let info = RouteInfo::new(0, "/users/{user_id:int}/posts/{post_id:int}", false);
        assert_eq!(info.match_pattern, "/users/{user_id}/posts/{post_id}");
        assert_eq!(info.get_param_type("user_id"), ParamType::Int);
        assert_eq!(info.get_param_type("post_id"), ParamType::Int);
    }

    #[test]
    fn test_route_info_mixed_types() {
        let info = RouteInfo::new(0, "/products/{id:int}/price/{value:float}", false);
        assert_eq!(info.get_param_type("id"), ParamType::Int);
        assert_eq!(info.get_param_type("value"), ParamType::Float);
    }

    #[test]
    fn test_route_info_root() {
        let info = RouteInfo::new(0, "/", false);
        assert_eq!(info.match_pattern, "/");
        assert!(info.param_types.is_empty());
    }
}
