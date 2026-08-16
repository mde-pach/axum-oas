//! A handler returning a type that implements `IntoResponse` (so plain axum
//! would accept it) but not `OperationOutput` must fail to register on an
//! `OasRouter`, with the curated diagnostic.

use axum::response::{IntoResponse, Response};

struct Undescribable;

impl IntoResponse for Undescribable {
    fn into_response(self) -> Response {
        ().into_response()
    }
}

async fn handler() -> Undescribable {
    Undescribable
}

fn main() {
    let _router: axum_oas::OasRouter<()> =
        axum_oas::OasRouter::new().route("/broken", axum_oas::get(handler));
}
