//! # axum-oas
//!
//! Zero-duplication OpenAPI 3.1 for [axum]: the handler is the single source
//! of truth. Typed extractors and response types go in; validation *and*
//! documentation come out — and anything the library cannot describe is a
//! **compile error**, never a silently under-documented spec.
//!
//! ```rust
//! use axum_oas::{Created, OasRouter, get, post};
//! use schemars::JsonSchema;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, JsonSchema)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! async fn list_users() -> axum::Json<Vec<User>> {
//!     axum::Json(vec![])
//! }
//!
//! async fn create_user(axum::Json(user): axum::Json<User>) -> Created<User> {
//!     Created(user)
//! }
//!
//! let app: axum::Router = OasRouter::new()
//!     .title("users")
//!     .version("0.1.0")
//!     .route("/users", get(list_users).post(create_user))
//!     .into_router(); // serves GET /openapi.json too
//! ```
//!
//! See the [README](https://github.com/mde-pach/axum-oas) for the
//! design (four pillars) and the honest v0 status.
//!
//! [axum]: https://docs.rs/axum

#![warn(missing_docs)]

mod operation;
mod response;
mod router;
pub mod routing;
pub mod spec;

pub use operation::{OperationInput, OperationOutput};
pub use response::{Created, NoContent, Ok};
pub use router::OasRouter;
pub use routing::{OasHandler, OasMethodRouter, delete, get, post, put};

// Re-exported so downstream code can name the exact versions axum-oas builds
// against.
pub use axum;
pub use schemars;
