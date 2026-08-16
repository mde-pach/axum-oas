//! A tiny CRUD-ish API demonstrating axum-oas.
//!
//! Run with `cargo run --example demo`, then:
//!
//! ```text
//! curl http://127.0.0.1:3000/openapi.json
//! curl http://127.0.0.1:3000/users
//! curl -X POST http://127.0.0.1:3000/users -H 'content-type: application/json' -d '{"name":"ada"}'
//! curl http://127.0.0.1:3000/users/1
//! curl -X DELETE http://127.0.0.1:3000/users/1 -i
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_oas::{Created, NoContent, OasRouter, OperationOutput, get};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, JsonSchema)]
struct User {
    id: u64,
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NewUser {
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListParams {
    /// Maximum number of users to return.
    limit: Option<usize>,
}

/// A domain error that is *itself* the single source of truth for its status
/// code and body: `IntoResponse` enforces it, `OperationOutput` documents it.
#[derive(Debug, Serialize, JsonSchema)]
struct NotFound {
    message: String,
}

impl IntoResponse for NotFound {
    fn into_response(self) -> Response {
        (StatusCode::NOT_FOUND, Json(self)).into_response()
    }
}

impl OperationOutput for NotFound {
    fn operation_output(
        operation: &mut axum_oas::spec::Operation,
        generator: &mut schemars::generate::SchemaGenerator,
    ) {
        let schema = generator.subschema_for::<NotFound>().to_value();
        operation.responses.insert(
            "404".to_owned(),
            axum_oas::spec::Response::json("Not Found", schema),
        );
    }
}

#[derive(Clone, Default)]
struct AppState {
    users: Arc<Mutex<HashMap<u64, User>>>,
    next_id: Arc<Mutex<u64>>,
}

async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<Vec<User>> {
    let users = state.users.lock().unwrap();
    let mut users: Vec<User> = users.values().cloned().collect();
    users.sort_by_key(|u| u.id);
    users.truncate(params.limit.unwrap_or(usize::MAX));
    Json(users)
}

async fn create_user(State(state): State<AppState>, Json(new): Json<NewUser>) -> Created<User> {
    let mut next_id = state.next_id.lock().unwrap();
    *next_id += 1;
    let user = User {
        id: *next_id,
        name: new.name,
    };
    state.users.lock().unwrap().insert(user.id, user.clone());
    Created(user)
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<axum_oas::Ok<User>, NotFound> {
    state
        .users
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .map(axum_oas::Ok)
        .ok_or_else(|| NotFound {
            message: format!("no user with id {id}"),
        })
}

async fn delete_user(State(state): State<AppState>, Path(id): Path<u64>) -> NoContent {
    state.users.lock().unwrap().remove(&id);
    NoContent
}

fn app() -> axum::Router {
    OasRouter::new()
        .title("axum-oas demo")
        .version(env!("CARGO_PKG_VERSION"))
        .description("A tiny CRUD-ish API whose OpenAPI 3.1 document is derived from its handlers")
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .into_router()
        .with_state(AppState::default())
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind 127.0.0.1:3000");
    println!("demo listening on http://127.0.0.1:3000 (spec at /openapi.json)");
    axum::serve(listener, app()).await.expect("server error");
}
