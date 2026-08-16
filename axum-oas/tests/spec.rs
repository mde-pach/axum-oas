//! Integration tests: build a small API and assert on the generated
//! OpenAPI 3.1 document served at `/openapi.json`, plus on the actual
//! runtime behaviour of the delegated axum router.

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Request, StatusCode};
use axum_oas::{Created, NoContent, OasRouter, get};
use http_body_util::BodyExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower::ServiceExt;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct Pet {
    id: u64,
    name: String,
    tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NewPet {
    name: String,
    tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ListParams {
    limit: Option<u32>,
    tag: Option<String>,
}

async fn list_pets(Query(_params): Query<ListParams>) -> Json<Vec<Pet>> {
    Json(vec![Pet {
        id: 1,
        name: "rex".into(),
        tag: None,
    }])
}

async fn create_pet(Json(new): Json<NewPet>) -> Created<Pet> {
    Created(Pet {
        id: 2,
        name: new.name,
        tag: new.tag,
    })
}

async fn get_pet(Path(id): Path<u64>) -> Json<Pet> {
    Json(Pet {
        id,
        name: "rex".into(),
        tag: None,
    })
}

async fn delete_pet(State(_db): State<()>, Path(_id): Path<u64>) -> NoContent {
    NoContent
}

fn app() -> axum::Router {
    OasRouter::new()
        .title("pets")
        .version("1.2.3")
        .route("/pets", get(list_pets).post(create_pet))
        .route("/pets/{id}", get(get_pet).delete(delete_pet))
        .into_router()
        .with_state(())
}

async fn fetch_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = match body {
        Some(body) => Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    };
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

async fn spec() -> Value {
    let (status, json) = fetch_json(app(), "GET", "/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    json
}

#[tokio::test]
async fn document_root_and_info() {
    let spec = spec().await;
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["info"]["title"], "pets");
    assert_eq!(spec["info"]["version"], "1.2.3");
}

#[tokio::test]
async fn all_paths_and_methods_present() {
    let spec = spec().await;
    let paths = spec["paths"].as_object().unwrap();
    assert_eq!(
        paths.keys().collect::<Vec<_>>(),
        vec!["/pets", "/pets/{id}"]
    );
    let mut pets_methods: Vec<&String> =
        spec["paths"]["/pets"].as_object().unwrap().keys().collect();
    pets_methods.sort();
    assert_eq!(pets_methods, vec!["get", "post"]);
    let mut id_methods: Vec<&String> = spec["paths"]["/pets/{id}"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    id_methods.sort();
    assert_eq!(id_methods, vec!["delete", "get"]);
}

#[tokio::test]
async fn request_body_schema_is_referenced_and_defined() {
    let spec = spec().await;
    let body = &spec["paths"]["/pets"]["post"]["requestBody"];
    assert_eq!(body["required"], true);
    assert_eq!(
        body["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/NewPet"
    );
    let new_pet = &spec["components"]["schemas"]["NewPet"];
    assert_eq!(new_pet["type"], "object");
    assert!(new_pet["properties"]["name"].is_object());
    assert!(new_pet["properties"]["tag"].is_object());
}

#[tokio::test]
async fn response_status_codes_and_schemas() {
    let spec = spec().await;

    // GET /pets -> 200 with Vec<Pet>
    let ok = &spec["paths"]["/pets"]["get"]["responses"]["200"];
    let schema = &ok["content"]["application/json"]["schema"];
    assert_eq!(schema["type"], "array");
    assert_eq!(schema["items"]["$ref"], "#/components/schemas/Pet");

    // POST /pets -> 201 (typed response Created<Pet>)
    let created = &spec["paths"]["/pets"]["post"]["responses"]["201"];
    assert_eq!(
        created["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Pet"
    );
    assert!(spec["paths"]["/pets"]["post"]["responses"]["200"].is_null());

    // DELETE /pets/{id} -> 204 with no content (typed response NoContent)
    let no_content = &spec["paths"]["/pets/{id}"]["delete"]["responses"]["204"];
    assert_eq!(no_content["description"], "No Content");
    assert!(no_content["content"].is_null());

    // The Pet component schema exists exactly once, in components.
    assert_eq!(spec["components"]["schemas"]["Pet"]["type"], "object");
}

#[tokio::test]
async fn query_parameters_from_typed_struct() {
    let spec = spec().await;
    let params = spec["paths"]["/pets"]["get"]["parameters"]
        .as_array()
        .unwrap();
    let names: Vec<&str> = params.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"limit"));
    assert!(names.contains(&"tag"));
    for p in params {
        assert_eq!(p["in"], "query");
        // Both fields are Option<_> -> not required.
        assert_eq!(p["required"], false);
    }
}

#[tokio::test]
async fn path_parameters_from_route_template() {
    let spec = spec().await;
    for method in ["get", "delete"] {
        let params = spec["paths"]["/pets/{id}"][method]["parameters"]
            .as_array()
            .unwrap();
        let id = params.iter().find(|p| p["name"] == "id").unwrap();
        assert_eq!(id["in"], "path");
        assert_eq!(id["required"], true);
        assert_eq!(id["schema"]["type"], "string");
    }
}

#[tokio::test]
async fn router_still_routes() {
    // The OasRouter must delegate to a fully functional axum Router.
    let (status, json) = fetch_json(app(), "GET", "/pets", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json[0]["name"], "rex");

    let (status, json) = fetch_json(
        app(),
        "POST",
        "/pets",
        Some(serde_json::json!({ "name": "milo" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json["name"], "milo");

    let (status, json) = fetch_json(app(), "GET", "/pets/42", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"], 42);

    let (status, body) = fetch_json(app(), "DELETE", "/pets/42", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);
}
