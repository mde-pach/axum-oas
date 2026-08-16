//! [`OasRouter`]: mirrors `axum::Router`'s `route(path, method_router)` API
//! while accumulating an OpenAPI 3.1 document, then lowers into a plain
//! `axum::Router` (plus a served `/openapi.json`) with [`into_router`].
//!
//! [`into_router`]: OasRouter::into_router

use std::collections::BTreeMap;

use axum::Router;
use schemars::generate::{SchemaGenerator, SchemaSettings};

use crate::routing::OasMethodRouter;
use crate::spec::{Info, OpenApi, Operation, Parameter, PathItem};

/// A router that accumulates an OpenAPI document while delegating routing to
/// a real [`axum::Router`].
#[derive(Debug)]
pub struct OasRouter<S = ()> {
    router: Router<S>,
    paths: BTreeMap<String, PathItem>,
    generator: SchemaGenerator,
    info: Info,
}

impl<S> Default for OasRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> OasRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create an empty router.
    ///
    /// Schemas are generated with JSON Schema 2020-12 settings (the OpenAPI
    /// 3.1 dialect); named schemas are collected under
    /// `#/components/schemas`.
    pub fn new() -> Self {
        let mut settings = SchemaSettings::draft2020_12();
        settings.meta_schema = None;
        settings.definitions_path = "/components/schemas".into();
        Self {
            router: Router::new(),
            paths: BTreeMap::new(),
            generator: SchemaGenerator::new(settings),
            info: Info::default(),
        }
    }

    /// Set the OpenAPI `info.title`.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.info.title = title.into();
        self
    }

    /// Set the OpenAPI `info.version`.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.info.version = version.into();
        self
    }

    /// Set the OpenAPI `info.description`.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    /// Register a route, exactly like [`axum::Router::route`], and document
    /// every operation captured by the [`OasMethodRouter`].
    ///
    /// Path parameter names are parsed from the axum route template
    /// (`/users/{id}`); in v0 each is documented with a `string` schema.
    #[must_use]
    pub fn route(mut self, path: &str, method_router: OasMethodRouter<S>) -> Self {
        let (doc_path, path_params) = parse_path_template(path);
        let item = self.paths.entry(doc_path).or_default();
        for (method, describe) in &method_router.operations {
            let mut operation = Operation::default();
            for name in &path_params {
                operation.parameters.push(Parameter {
                    name: name.clone(),
                    location: "path",
                    required: true,
                    schema: serde_json::json!({ "type": "string" }),
                });
            }
            describe(&mut operation, &mut self.generator);
            item.insert(method, operation);
        }
        self.router = self.router.route(path, method_router.inner);
        self
    }

    /// Consume the router, returning the underlying [`axum::Router`] and the
    /// accumulated [`OpenApi`] document — without serving `/openapi.json`.
    pub fn into_parts(mut self) -> (Router<S>, OpenApi) {
        let mut doc = OpenApi {
            info: self.info,
            paths: self.paths,
            ..OpenApi::default()
        };
        doc.components.schemas = self.generator.take_definitions(true);
        (self.router, doc)
    }

    /// Consume the router, returning a plain [`axum::Router`] that also
    /// serves the accumulated document at `GET /openapi.json`.
    pub fn into_router(self) -> Router<S> {
        let (router, doc) = self.into_parts();
        let json =
            serde_json::to_value(&doc).expect("BUG: the OpenAPI document model always serializes");
        router.route(
            "/openapi.json",
            axum::routing::get(move || async move { axum::Json(json) }),
        )
    }
}

/// Parse an axum 0.8 route template into an OpenAPI path plus the ordered
/// list of path parameter names.
///
/// `/users/{id}` -> (`/users/{id}`, `["id"]`);
/// wildcard `/files/{*rest}` -> (`/files/{rest}`, `["rest"]`).
fn parse_path_template(path: &str) -> (String, Vec<String>) {
    let mut params = Vec::new();
    let doc_path = path
        .split('/')
        .map(
            |segment| match segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Some(name) => {
                    let name = name.strip_prefix('*').unwrap_or(name);
                    params.push(name.to_owned());
                    format!("{{{name}}}")
                }
                None => segment.to_owned(),
            },
        )
        .collect::<Vec<_>>()
        .join("/");
    (doc_path, params)
}

#[cfg(test)]
mod tests {
    use super::parse_path_template;

    #[test]
    fn plain_path() {
        assert_eq!(parse_path_template("/users"), ("/users".into(), vec![]));
    }

    #[test]
    fn one_param() {
        assert_eq!(
            parse_path_template("/users/{id}"),
            ("/users/{id}".into(), vec!["id".into()])
        );
    }

    #[test]
    fn wildcard_param() {
        assert_eq!(
            parse_path_template("/files/{*rest}"),
            ("/files/{rest}".into(), vec!["rest".into()])
        );
    }

    #[test]
    fn multiple_params() {
        assert_eq!(
            parse_path_template("/orgs/{org}/repos/{repo}"),
            (
                "/orgs/{org}/repos/{repo}".into(),
                vec!["org".into(), "repo".into()]
            )
        );
    }
}
