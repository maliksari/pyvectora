//! # `PyVectora` Macros
//!
//! Procedural macros for the `PyVectora` framework.
//! Reserved for future route decorator macros.
//!
//! ## Planned Features
//!
//! - `#[route(GET, "/path")]` attribute macro
//! - `#[middleware]` for tower layer generation
//! - `#[validator]` for Pydantic-style validation

use proc_macro::TokenStream;

/// Placeholder macro for route definition (to be implemented in FAZ 2)
///
/// # Usage (planned)
///
/// ```ignore
/// #[route(GET, "/users/{id}")]
/// fn get_user(id: i32) -> Response {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
