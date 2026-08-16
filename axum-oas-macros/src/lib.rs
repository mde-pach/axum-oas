//! # axum-oas-macros
//!
//! **Empty stub crate — reserved for the future inert `#[api]` doc-comment
//! attribute.**
//!
//! In a later release this crate will provide an *inert* `#[api]` attribute
//! that lifts Rust doc comments (`///`) into the OpenAPI `summary` /
//! `description` fields of the operation, without ever redeclaring routes,
//! parameters, schemas, or status codes (those stay derived from the types).
//!
//! It intentionally exports no macros in v0 so that `axum-oas` carries no
//! proc-macro compile-time cost until the attribute actually exists.
