//! Typed responses: the status code lives in the type, so it can be both
//! *enforced at runtime* (`IntoResponse`) and *documented at compile time*
//! (`OperationOutput`) from a single declaration.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use schemars::JsonSchema;
use schemars::generate::SchemaGenerator;
use serde::Serialize;

use crate::operation::OperationOutput;
use crate::spec;
use crate::spec::Operation;

/// `200 OK` with a JSON body of `T`.
///
/// Note: this type shadows `Result::Ok` in value position when imported
/// unqualified. Either import it as `use axum_oas::Ok as OkJson;` or refer to
/// it as `axum_oas::Ok<T>`.
#[derive(Debug, Clone)]
pub struct Ok<T>(pub T);

/// `201 Created` with a JSON body of `T`.
#[derive(Debug, Clone)]
pub struct Created<T>(pub T);

/// `204 No Content`, no body.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContent;

impl<T: Serialize> IntoResponse for Ok<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, axum::Json(self.0)).into_response()
    }
}

impl<T: JsonSchema> OperationOutput for Ok<T> {
    fn operation_output(operation: &mut Operation, generator: &mut SchemaGenerator) {
        let schema = generator.subschema_for::<T>().to_value();
        operation
            .responses
            .insert("200".to_owned(), spec::Response::json("OK", schema));
    }
}

impl<T: Serialize> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, axum::Json(self.0)).into_response()
    }
}

impl<T: JsonSchema> OperationOutput for Created<T> {
    fn operation_output(operation: &mut Operation, generator: &mut SchemaGenerator) {
        let schema = generator.subschema_for::<T>().to_value();
        operation
            .responses
            .insert("201".to_owned(), spec::Response::json("Created", schema));
    }
}

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        let mut response = StatusCode::NO_CONTENT.into_response();
        // Be explicit that there is no body.
        response.headers_mut().remove(header::CONTENT_TYPE);
        response
    }
}

impl OperationOutput for NoContent {
    fn operation_output(operation: &mut Operation, _generator: &mut SchemaGenerator) {
        operation
            .responses
            .insert("204".to_owned(), spec::Response::empty("No Content"));
    }
}
