//! # HTTP Request
//!
//! High-performance request wrapper with lazy parsing.
//!
//! ## Design Principles (SOLID)
//!
//! - **S**: Request only handles request data, not response
//! - **O**: Extensible via new methods without breaking changes
//! - **D**: Does not expose hyper types to Python layer

use crate::error::Result;
use crate::router::Method;
use crate::types::ParamValue;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::Request;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyString};
use serde_json::Value;
use std::collections::HashMap;

/// HTTP Request wrapper for Python interop
///
/// Provides lazy access to request components:
/// - Headers are stored but accessed on-demand
/// - Body is collected once and cached
/// - Query string is parsed on first access
#[pyclass(name = "Request", dict)]
#[derive(Debug, Clone)]
pub struct PyRequest {
    /// HTTP method
    pub method: Method,
    /// Request path (without query string)
    pub path: String,
    /// Raw query string (e.g., "page=1&limit=10")
    query_string: Option<String>,
    /// Parsed query parameters (lazy)
    query_params: HashMap<String, String>,
    /// Typed path parameters (FAZ 2)
    pub typed_params: HashMap<String, ParamValue>,
    /// Request headers
    headers: hyper::HeaderMap,
    /// Request body (collected)
    body: Option<Bytes>,
    /// Validated JWT claims
    pub claims: Option<Value>,
}

#[pymethods]
impl PyRequest {
    /// Get the HTTP method
    #[getter]
    fn method(&self) -> String {
        self.method.to_string()
    }

    /// Get the request path
    #[getter]
    fn path(&self) -> String {
        self.path.clone()
    }

    /// Get path parameters as a dict with typed values
    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.typed_params {
            match v {
                ParamValue::String(s) => dict.set_item(k, s)?,
                ParamValue::Int(i) => dict.set_item(k, *i)?,
                ParamValue::Float(f) => dict.set_item(k, *f)?,
                ParamValue::Bool(b) => dict.set_item(k, *b)?,
            }
        }
        Ok(dict.into())
    }

    /// Get query string parameters as a dict
    #[getter]
    fn query(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.query_params {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get all request headers as a dict
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            if let Ok(val) = v.to_str() {
                dict.set_item(k.as_str(), val)?;
            }
        }
        Ok(dict.into())
    }

    /// Get the request body as bytes
    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.body {
            Some(b) => Ok(PyBytes::new(py, b).into()),
            None => Ok(py.None()),
        }
    }

    /// Get the request body as text (UTF-8)
    #[getter]
    fn text(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.body {
            Some(b) => match std::str::from_utf8(b) {
                Ok(s) => Ok(PyString::new(py, s).into()),
                Err(_) => Ok(py.None()),
            },
            None => Ok(py.None()),
        }
    }

    /// Parse request body as JSON
    fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.body {
            Some(b) => {
                let json_module = py.import("json")?;
                let body_bytes = PyBytes::new(py, b);
                Ok(json_module.call_method1("loads", (body_bytes,))?.into())
            }
            None => Ok(PyDict::new(py).into()),
        }
    }
}

impl PyRequest {
    /// Create a new PyRequest manually (for testing/internal use)
    pub fn new(
        method: Method,
        path: String,
        headers_map: HashMap<String, String>,
        body: Option<Bytes>,
    ) -> Self {
        let (path, query_string) = if let Some((p, q)) = path.split_once('?') {
            (p.to_string(), Some(q.to_string()))
        } else {
            (path, None)
        };

        let query_params = parse_query_string(query_string.as_deref());

        let mut headers = hyper::HeaderMap::new();
        for (k, v) in headers_map {
            if let (Ok(n), Ok(v)) = (
                hyper::header::HeaderName::from_bytes(k.as_bytes()),
                hyper::header::HeaderValue::from_str(&v),
            ) {
                headers.insert(n, v);
            }
        }

        Self {
            method,
            path,
            query_string,
            query_params,
            typed_params: HashMap::new(),
            headers,
            body,
            claims: None,
        }
    }

    /// Create from hyper request
    pub async fn from_hyper(req: Request<hyper::body::Incoming>) -> Result<Self> {
        Self::from_hyper_with_limit(req, usize::MAX).await
    }

    /// Create from hyper request with body size limit
    pub async fn from_hyper_with_limit(
        req: Request<hyper::body::Incoming>,
        max_body_size: usize,
    ) -> Result<Self> {
        let method = match *req.method() {
            hyper::Method::GET => Method::Get,
            hyper::Method::POST => Method::Post,
            hyper::Method::PUT => Method::Put,
            hyper::Method::DELETE => Method::Delete,
            hyper::Method::PATCH => Method::Patch,
            hyper::Method::HEAD => Method::Head,
            hyper::Method::OPTIONS => Method::Options,
            _ => Method::Get, // Fallback
        };

        let uri = req.uri();
        let path = uri.path().to_string();
        let query_string = uri.query().map(String::from);

        let query_params = parse_query_string(query_string.as_deref());

        let headers = req.headers().clone();
        if let Some(len) = headers.get(hyper::header::CONTENT_LENGTH) {
            if let Ok(len_str) = len.to_str() {
                if let Ok(content_len) = len_str.parse::<usize>() {
                    if content_len > max_body_size {
                        return Err(crate::error::Error::PayloadTooLarge {
                            limit: max_body_size,
                            actual: content_len,
                        });
                    }
                }
            }
        }

        let body = match BodyExt::collect(req.into_body()).await {
            Ok(collected) => {
                let bytes = collected.to_bytes();
                if bytes.len() > max_body_size {
                    return Err(crate::error::Error::PayloadTooLarge {
                        limit: max_body_size,
                        actual: bytes.len(),
                    });
                }
                Some(bytes)
            }
            Err(_) => None,
        };

        Ok(Self {
            method,
            path,
            query_string,
            query_params,
            headers,
            body,
            typed_params: HashMap::new(),
            claims: None,
        })
    }

    /// Get a header value by name (case-insensitive)
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Set or override a header
    pub fn set_header(&mut self, name: &str, value: &str) {
        if let (Ok(n), Ok(v)) = (
            hyper::header::HeaderName::from_bytes(name.as_bytes()),
            hyper::header::HeaderValue::from_str(value),
        ) {
            self.headers.insert(n, v);
        }
    }

    /// Get all headers as a HashMap
    #[must_use]
    pub fn headers_map(&self) -> HashMap<String, String> {
        self.headers
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|val| (k.as_str().to_string(), val.to_string()))
            })
            .collect()
    }

    /// Get query parameters as a HashMap
    #[must_use]
    pub fn query_map(&self) -> &HashMap<String, String> {
        &self.query_params
    }

    /// Get raw query string
    #[must_use]
    pub fn query_string(&self) -> Option<&str> {
        self.query_string.as_deref()
    }

    /// Get the request body as bytes (Rust)
    #[must_use]
    pub fn body_bytes(&self) -> Option<&[u8]> {
        self.body.as_ref().map(|b| b.as_ref())
    }

    /// Get the request body as string (UTF-8)
    #[must_use]
    pub fn body_str(&self) -> Option<&str> {
        self.body_bytes().and_then(|b| std::str::from_utf8(b).ok())
    }
}

/// Parse query string into HashMap
///
/// Handles URL decoding and duplicate keys (last value wins).
fn parse_query_string(query: Option<&str>) -> HashMap<String, String> {
    query
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?;
                    let value = parts.next().unwrap_or("");
                    let key = url_decode(key);
                    let value = url_decode(value);
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Basic URL decoding
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(byte as char);
                    } else {
                        result.push('%');
                        result.push_str(&hex);
                    }
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_string_simple() {
        let result = parse_query_string(Some("page=1&limit=10"));
        assert_eq!(result.get("page"), Some(&"1".to_string()));
        assert_eq!(result.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_parse_query_string_empty() {
        let result = parse_query_string(None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_query_string_url_encoded() {
        let result = parse_query_string(Some("name=John+Doe&city=New%20York"));
        assert_eq!(result.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(result.get("city"), Some(&"New York".to_string()));
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello+world"), "hello world");
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("100%25"), "100%");
    }
}
