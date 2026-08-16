//! The two describability traits, and their implementations for axum's
//! built-in extractors.
//!
//! These traits are the enforcement point of axum-oas's core rule:
//! **compile-error over silent omission**. If a handler uses an extractor or
//! return type that axum-oas cannot describe, the route registration fails to
//! compile with a curated diagnostic (via `#[diagnostic::on_unimplemented]`,
//! stable since Rust 1.78) instead of producing an under-documented spec.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use schemars::generate::{SchemaGenerator, SchemaSettings};

use crate::spec::{Operation, Parameter, RequestBody};

/// A handler *input* (extractor) that axum-oas knows how to document.
///
/// Implemented for the extractors supported in v0:
///
/// | Extractor | Documented as |
/// |---|---|
/// | [`axum::Json<T>`] (`T: JsonSchema`) | `application/json` request body |
/// | [`axum::extract::Query<T>`] (`T: JsonSchema`) | one `query` parameter per field |
/// | [`axum::extract::Path<T>`] | no-op (path parameter *names* come from the route template at registration; v0 types them as `string`) |
/// | [`axum::extract::State<S>`] | no-op (not part of the HTTP interface) |
/// | [`axum::Extension<T>`] | no-op (not part of the HTTP interface) |
/// | [`axum::http::HeaderMap`] | no-op |
/// | `Option<T>` (`T: OperationInput`) | delegates to `T` |
///
/// Implement it for your own extractors to make them describable.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be described in the OpenAPI document, so this handler cannot be registered on an `OasRouter`",
    label = "this extractor is not describable",
    note = "axum-oas refuses to silently omit parts of a handler from the generated spec (compile-error over silent omission)",
    note = "for JSON bodies use `axum::Json<T>` and for query strings use `axum::extract::Query<T>`, with `T` deriving `schemars::JsonSchema`",
    note = "for extractors that are not part of the HTTP interface, or your own extractors, implement `axum_oas::OperationInput` (it is a no-op impl for non-HTTP inputs like `State`)"
)]
pub trait OperationInput {
    /// Describe this extractor's contribution (parameters, request body) into
    /// `operation`, registering any named schemas on `generator`.
    fn operation_input(operation: &mut Operation, generator: &mut SchemaGenerator);
}

/// A handler *return type* that axum-oas knows how to document.
///
/// Implemented for the return types supported in v0:
///
/// | Return type | Documented as |
/// |---|---|
/// | [`axum::Json<T>`] (`T: JsonSchema`) | `200` with JSON body `T` |
/// | [`crate::Ok<T>`] (`T: JsonSchema`) | `200` with JSON body `T` |
/// | [`crate::Created<T>`] (`T: JsonSchema`) | `201` with JSON body `T` |
/// | [`crate::NoContent`] | `204`, no body |
/// | `Result<T, E>` (both `OperationOutput`) | union of both variants' responses |
///
/// Implement it (together with `IntoResponse`) for your own response types.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be described in the OpenAPI document, so this handler cannot be registered on an `OasRouter`",
    label = "this return type is not describable",
    note = "axum-oas refuses to silently omit a handler's response from the generated spec (compile-error over silent omission)",
    note = "return `axum::Json<T>` or a typed response like `axum_oas::Ok<T>`, `axum_oas::Created<T>`, or `axum_oas::NoContent`, with `T` deriving `schemars::JsonSchema`",
    note = "or implement `axum_oas::OperationOutput` for `{Self}` to declare its status codes and body schema yourself"
)]
pub trait OperationOutput {
    /// Describe this return type's responses into `operation`, registering
    /// any named schemas on `generator`.
    fn operation_output(operation: &mut Operation, generator: &mut SchemaGenerator);
}

// ---------------------------------------------------------------------------
// OperationInput impls for axum extractors
// ---------------------------------------------------------------------------

impl<T: JsonSchema> OperationInput for axum::Json<T> {
    fn operation_input(operation: &mut Operation, generator: &mut SchemaGenerator) {
        let schema = generator.subschema_for::<T>().to_value();
        operation.request_body = Some(RequestBody::json(schema));
    }
}

impl<T: JsonSchema> OperationInput for axum::extract::Query<T> {
    fn operation_input(operation: &mut Operation, _generator: &mut SchemaGenerator) {
        // Query parameters must be inlined (one OpenAPI parameter per field),
        // so we use a dedicated inlining generator rather than the shared one.
        let mut settings = SchemaSettings::draft2020_12();
        settings.meta_schema = None;
        settings.inline_subschemas = true;
        let root = SchemaGenerator::new(settings)
            .root_schema_for::<T>()
            .to_value();

        let Some(obj) = root.as_object() else {
            return;
        };
        let required: BTreeSet<&str> = obj
            .get("required")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let Some(props) = obj.get("properties").and_then(|p| p.as_object()) else {
            return;
        };
        for (name, schema) in props {
            operation.parameters.push(Parameter {
                name: name.clone(),
                location: "query",
                required: required.contains(name.as_str()),
                schema: schema.clone(),
            });
        }
    }
}

/// No-op: path parameter *names* are taken from the route template at
/// registration time (`OasRouter::route`). In v0 they are documented with a
/// `string` schema; deriving per-parameter schemas from `T` is future work.
impl<T> OperationInput for axum::extract::Path<T> {
    fn operation_input(_operation: &mut Operation, _generator: &mut SchemaGenerator) {}
}

/// No-op: application state is not part of the HTTP interface.
impl<S> OperationInput for axum::extract::State<S> {
    fn operation_input(_operation: &mut Operation, _generator: &mut SchemaGenerator) {}
}

/// No-op: request extensions are not part of the HTTP interface.
impl<T> OperationInput for axum::Extension<T> {
    fn operation_input(_operation: &mut Operation, _generator: &mut SchemaGenerator) {}
}

/// No-op: v0 does not document individual headers.
impl OperationInput for axum::http::HeaderMap {
    fn operation_input(_operation: &mut Operation, _generator: &mut SchemaGenerator) {}
}

/// Delegates to `T`. Note that v0 does not (yet) downgrade `T`'s required
/// parameters to optional.
impl<T: OperationInput> OperationInput for Option<T> {
    fn operation_input(operation: &mut Operation, generator: &mut SchemaGenerator) {
        T::operation_input(operation, generator);
    }
}

// ---------------------------------------------------------------------------
// OperationOutput impls
// ---------------------------------------------------------------------------

impl<T: JsonSchema> OperationOutput for axum::Json<T> {
    fn operation_output(operation: &mut Operation, generator: &mut SchemaGenerator) {
        let schema = generator.subschema_for::<T>().to_value();
        operation
            .responses
            .insert("200".to_owned(), crate::spec::Response::json("OK", schema));
    }
}

/// The union of both variants' responses.
impl<T: OperationOutput, E: OperationOutput> OperationOutput for Result<T, E> {
    fn operation_output(operation: &mut Operation, generator: &mut SchemaGenerator) {
        T::operation_output(operation, generator);
        E::operation_output(operation, generator);
    }
}
