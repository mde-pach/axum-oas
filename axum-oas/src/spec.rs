//! Minimal serializable model of the subset of OpenAPI 3.1 that axum-oas
//! emits in v0.
//!
//! This is intentionally *not* a full OpenAPI object model: it only contains
//! the fields the router can actually derive from types. `BTreeMap` is used
//! throughout so the serialized document is deterministic.

// The field names map 1:1 to the OpenAPI 3.1 specification; per-field docs
// would only restate it.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// Root OpenAPI 3.1 document.
#[derive(Debug, Clone, Serialize)]
pub struct OpenApi {
    pub openapi: &'static str,
    pub info: Info,
    pub paths: BTreeMap<String, PathItem>,
    #[serde(skip_serializing_if = "Components::is_empty")]
    pub components: Components,
}

impl Default for OpenApi {
    fn default() -> Self {
        Self {
            openapi: "3.1.0",
            info: Info::default(),
            paths: BTreeMap::new(),
            components: Components::default(),
        }
    }
}

/// `info` object.
#[derive(Debug, Clone, Serialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for Info {
    fn default() -> Self {
        Self {
            title: "API".to_owned(),
            version: "0.0.0".to_owned(),
            description: None,
        }
    }
}

/// `components` object (only `schemas` in v0).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Components {
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub schemas: serde_json::Map<String, Value>,
}

impl Components {
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}

/// A path item: HTTP method (lowercase) -> operation.
pub type PathItem = BTreeMap<&'static str, Operation>;

/// A single OpenAPI operation, built up by
/// [`OperationInput`](crate::OperationInput) /
/// [`OperationOutput`](crate::OperationOutput) implementations.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    pub responses: BTreeMap<String, Response>,
}

/// An OpenAPI parameter (`query` or `path` in v0).
#[derive(Debug, Clone, Serialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: &'static str,
    pub required: bool,
    pub schema: Value,
}

/// An OpenAPI request body (always `application/json` in v0).
#[derive(Debug, Clone, Serialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: BTreeMap<&'static str, MediaType>,
}

/// An OpenAPI response.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub content: BTreeMap<&'static str, MediaType>,
}

/// A media type object holding a schema.
#[derive(Debug, Clone, Serialize)]
pub struct MediaType {
    pub schema: Value,
}

impl RequestBody {
    /// A required `application/json` body with the given schema.
    pub fn json(schema: Value) -> Self {
        Self {
            required: true,
            content: BTreeMap::from([("application/json", MediaType { schema })]),
        }
    }
}

impl Response {
    /// A response with an `application/json` body.
    pub fn json(description: impl Into<String>, schema: Value) -> Self {
        Self {
            description: description.into(),
            content: BTreeMap::from([("application/json", MediaType { schema })]),
        }
    }

    /// A body-less response.
    pub fn empty(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            content: BTreeMap::new(),
        }
    }
}
