//! # High-Performance Router
//!
//! Radix-trie based router using `matchit` for O(log n) route matching.
//! Significantly faster than regex-based routing.
//!
//! ## Features
//!
//! - Path parameter extraction (`/users/{id}`)
//! - Typed parameters (`/users/{id:int}`)
//! - Wildcard routes (`/files/*path`)
//! - Zero-copy path matching
//!
//! ## SOLID Principles
//!
//! - **S**: Router only handles routing, type conversion delegated to `types` module
//! - **O**: Extensible via new ParamType without modifying Router
//! - **D**: Depends on `types::convert_param`, not concrete conversion logic

use crate::error::{Error, Result};
use crate::route::RouteInfo;
use crate::types::{convert_param, ParamValue};
use matchit::Router as MatchitRouter;
use std::collections::HashMap;

/// HTTP methods supported by the router
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// HTTP GET
    Get,
    /// HTTP POST
    Post,
    /// HTTP PUT
    Put,
    /// HTTP DELETE
    Delete,
    /// HTTP PATCH
    Patch,
    /// HTTP HEAD
    Head,
    /// HTTP OPTIONS
    Options,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Patch => write!(f, "PATCH"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

/// Route handler identifier
pub type HandlerId = usize;

/// Matched route with extracted and converted parameters
///
/// This struct contains both raw string params (backward compatible)
/// and typed params (new FAZ 2 feature).
#[derive(Debug)]
pub struct Match<'a> {
    /// The handler ID for this route
    pub handler_id: HandlerId,
    /// Raw extracted path parameters (backward compatible)
    pub params: HashMap<&'a str, &'a str>,
    /// Typed path parameters (FAZ 2 feature)
    pub typed_params: HashMap<String, ParamValue>,
    /// Whether authentication is required (Phase 4)
    pub auth_required: bool,
}

impl<'a> Match<'a> {
    /// Get a typed parameter by name
    ///
    /// Returns `None` if parameter doesn't exist or conversion failed.
    #[must_use]
    pub fn get_typed(&self, name: &str) -> Option<&ParamValue> {
        self.typed_params.get(name)
    }

    /// Get a parameter as i64 (convenience method)
    #[must_use]
    pub fn get_int(&self, name: &str) -> Option<i64> {
        self.typed_params.get(name).and_then(ParamValue::as_int)
    }

    /// Get a parameter as f64 (convenience method)
    #[must_use]
    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.typed_params.get(name).and_then(ParamValue::as_float)
    }

    /// Get a parameter as bool (convenience method)
    #[must_use]
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.typed_params.get(name).and_then(ParamValue::as_bool)
    }
}

/// Per-method storage for routes
#[derive(Clone)]
struct MethodRoutes {
    /// Matchit router for path matching
    router: MatchitRouter<HandlerId>,
    /// Route metadata indexed by handler ID
    routes: Vec<RouteInfo>,
}

impl MethodRoutes {
    fn new() -> Self {
        Self {
            router: MatchitRouter::new(),
            routes: Vec::new(),
        }
    }
}

/// High-performance HTTP router using radix trie
///
/// ## Design (SOLID)
///
/// - Single Responsibility: Only handles route registration and matching
/// - Open/Closed: Extensible via new ParamTypes without modification
/// - Dependency Inversion: Uses abstract `convert_param` function
#[derive(Clone)]
pub struct Router {
    /// Per-method routers for efficient matching
    method_routes: HashMap<Method, MethodRoutes>,
    /// Counter for generating handler IDs
    next_handler_id: HandlerId,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Create a new empty router
    #[must_use]
    pub fn new() -> Self {
        Self {
            method_routes: HashMap::new(),
            next_handler_id: 0,
        }
    }

    /// Register a route with the given method and path pattern
    ///
    /// Supports typed parameters: `/users/{id:int}`, `/products/{price:float}`
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method
    /// * `path` - Path pattern (e.g., "/users/{id}" or "/users/{id:int}")
    ///
    /// # Returns
    ///
    /// The handler ID assigned to this route
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidRoutePattern` if the pattern is malformed
    /// Returns `Error::InvalidRoutePattern` if the pattern is malformed
    pub fn add_route(
        &mut self,
        method: Method,
        path: &str,
        auth_required: bool,
    ) -> Result<HandlerId> {
        let handler_id = self.next_handler_id;
        self.next_handler_id += 1;

        let route_info = RouteInfo::new(handler_id, path, auth_required);
        let match_pattern = route_info.match_pattern.clone();

        let method_routes = self
            .method_routes
            .entry(method)
            .or_insert_with(MethodRoutes::new);

        method_routes
            .router
            .insert(&match_pattern, handler_id)
            .map_err(|e| Error::InvalidRoutePattern {
                pattern: path.to_string(),
                reason: e.to_string(),
            })?;

        method_routes.routes.push(route_info);

        Ok(handler_id)
    }

    /// Match a request path against registered routes
    ///
    /// Returns both raw string params (backward compatible) and
    /// typed params based on route definition.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method of the request
    /// * `path` - Request path to match
    ///
    /// # Returns
    ///
    /// A `Match` containing the handler ID and both raw and typed parameters
    ///
    /// # Errors
    ///
    /// Returns `Error::RouteNotFound` if no matching route exists
    pub fn match_route<'a>(&'a self, method: Method, path: &'a str) -> Result<Match<'a>> {
        let method_routes =
            self.method_routes
                .get(&method)
                .ok_or_else(|| Error::RouteNotFound {
                    path: path.to_string(),
                })?;

        let matched = method_routes
            .router
            .at(path)
            .map_err(|_| Error::RouteNotFound {
                path: path.to_string(),
            })?;

        let handler_id = *matched.value;

        let route_info = method_routes
            .routes
            .iter()
            .find(|r| r.handler_id == handler_id)
            .ok_or_else(|| Error::RouteNotFound {
                path: path.to_string(),
            })?;

        let params: HashMap<&str, &str> = matched.params.iter().collect();

        let mut typed_params = HashMap::new();
        for (name, value) in &params {
            let param_type = route_info.get_param_type(name);
            let typed_value = convert_param(value, param_type)
                .unwrap_or_else(|_| ParamValue::String((*value).to_string()));
            typed_params.insert((*name).to_string(), typed_value);
        }

        Ok(Match {
            handler_id,
            params,
            typed_params,
            auth_required: route_info.auth_required,
        })
    }

    /// Convenience method to add a GET route
    pub fn get(&mut self, path: &str) -> Result<HandlerId> {
        self.add_route(Method::Get, path, false)
    }

    /// Convenience method to add a POST route
    pub fn post(&mut self, path: &str) -> Result<HandlerId> {
        self.add_route(Method::Post, path, false)
    }

    /// Convenience method to add a PUT route
    pub fn put(&mut self, path: &str) -> Result<HandlerId> {
        self.add_route(Method::Put, path, false)
    }

    /// Convenience method to add a DELETE route
    pub fn delete(&mut self, path: &str) -> Result<HandlerId> {
        self.add_route(Method::Delete, path, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_routing() {
        let mut router = Router::new();

        let id1 = router.get("/").unwrap();
        let id2 = router.get("/users").unwrap();
        let id3 = router.post("/users").unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);

        let m = router.match_route(Method::Get, "/").unwrap();
        assert_eq!(m.handler_id, 0);

        let m = router.match_route(Method::Get, "/users").unwrap();
        assert_eq!(m.handler_id, 1);

        let m = router.match_route(Method::Post, "/users").unwrap();
        assert_eq!(m.handler_id, 2);
    }

    #[test]
    fn test_path_parameters() {
        let mut router = Router::new();
        router.get("/users/{id}").unwrap();
        router.get("/users/{user_id}/posts/{post_id}").unwrap();

        let m = router.match_route(Method::Get, "/users/123").unwrap();
        assert_eq!(m.params.get("id"), Some(&"123"));

        let m = router
            .match_route(Method::Get, "/users/456/posts/789")
            .unwrap();
        assert_eq!(m.params.get("user_id"), Some(&"456"));
        assert_eq!(m.params.get("post_id"), Some(&"789"));
    }

    #[test]
    fn test_typed_int_parameter() {
        let mut router = Router::new();
        router.get("/users/{id:int}").unwrap();

        let m = router.match_route(Method::Get, "/users/123").unwrap();

        assert_eq!(m.params.get("id"), Some(&"123"));

        assert_eq!(m.typed_params.get("id"), Some(&ParamValue::Int(123)));
        assert_eq!(m.get_int("id"), Some(123));
    }

    #[test]
    fn test_typed_float_parameter() {
        let mut router = Router::new();
        router.get("/products/{price:float}").unwrap();

        let m = router.match_route(Method::Get, "/products/19.99").unwrap();

        assert_eq!(m.typed_params.get("price"), Some(&ParamValue::Float(19.99)));
        assert_eq!(m.get_float("price"), Some(19.99));
    }

    #[test]
    fn test_typed_bool_parameter() {
        let mut router = Router::new();
        router.get("/feature/{enabled:bool}").unwrap();

        let m = router.match_route(Method::Get, "/feature/true").unwrap();
        assert_eq!(m.get_bool("enabled"), Some(true));

        let m = router.match_route(Method::Get, "/feature/false").unwrap();
        assert_eq!(m.get_bool("enabled"), Some(false));
    }

    #[test]
    fn test_mixed_typed_parameters() {
        let mut router = Router::new();
        router.get("/orders/{id:int}/status/{active:bool}").unwrap();

        let m = router
            .match_route(Method::Get, "/orders/42/status/true")
            .unwrap();

        assert_eq!(m.get_int("id"), Some(42));
        assert_eq!(m.get_bool("active"), Some(true));
    }

    #[test]
    fn test_invalid_type_fallback_to_string() {
        let mut router = Router::new();
        router.get("/users/{id:int}").unwrap();

        let m = router.match_route(Method::Get, "/users/abc").unwrap();
        assert_eq!(
            m.typed_params.get("id"),
            Some(&ParamValue::String("abc".to_string()))
        );
    }

    #[test]
    fn test_route_not_found() {
        let router = Router::new();
        let result = router.match_route(Method::Get, "/nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::new();
        router.get("/users").unwrap();

        let result = router.match_route(Method::Post, "/users");
        assert!(result.is_err());
    }
}
