//! A handler using an extractor that plain axum accepts (`Method` implements
//! `FromRequestParts`) but that axum-oas cannot describe must fail to
//! register on an `OasRouter`, with the curated diagnostic.

use axum::http::Method;

async fn handler(_method: Method) -> axum_oas::NoContent {
    axum_oas::NoContent
}

fn main() {
    let _router: axum_oas::OasRouter<()> =
        axum_oas::OasRouter::new().route("/broken", axum_oas::get(handler));
}
