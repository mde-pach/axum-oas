# axum-oas

Zero-duplication OpenAPI 3.1 for [axum](https://github.com/tokio-rs/axum). The handler is the single source of truth: typed extractors and typed responses go in, the specification comes out — and anything the library cannot describe is a **compile error**, never a silently under-documented spec.

```rust
use axum_oas::{Created, OasRouter, get, post};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct User { id: u64, name: String }

async fn list_users() -> axum::Json<Vec<User>> { axum::Json(vec![]) }

async fn create_user(axum::Json(user): axum::Json<User>) -> Created<User> {
    Created(user)
}

let app: axum::Router = OasRouter::new()
    .title("users")
    .version("0.1.0")
    .route("/users", get(list_users).post(create_user))
    .into_router(); // also serves GET /openapi.json
```

No `#[path(...)]` attribute re-describing the handler. No response list repeated next to a function that already states its return type. The route is declared once.

## The four pillars

**1 · Type capture at route registration.** `axum_oas::get(handler)` bounds `H: Handler<T, S> + OasHandler<T, S>` — axum checks the handler *runs*, axum-oas checks it is *describable*, over the same extractor tuple `T`, resolved from one call site. The concrete return type axum erases is recovered through the `F: FnOnce(..) -> Fut` bound. This is the approach axum's own maintainers explored and endorsed, carried to its conclusion.

**2 · Compile-error over silent omission.** An extractor or return type that cannot be documented fails to compile, with a curated diagnostic (`#[diagnostic::on_unimplemented]`, stable since Rust 1.78):

```
error[E0277]: `Undescribable` cannot be described in the OpenAPI document,
              so this handler cannot be registered on an `OasRouter`
  --> src/main.rs:21:57
   |
21 |     OasRouter::new().route("/broken", axum_oas::get(handler));
   |                                       ------------- ^^^^^^^ this return type is not describable
   |
   = note: axum-oas refuses to silently omit a handler's response from the
           generated spec (compile-error over silent omission)
   = note: return `axum::Json<T>` or a typed response like `axum_oas::Ok<T>`,
           `axum_oas::Created<T>`, or `axum_oas::NoContent`
```

A spec that compiles is a spec that matches the code. This is the property no existing crate offers.

**3 · Status codes live in types.** `Ok<T>` → 200, `Created<T>` → 201, `NoContent` → 204, `Result<T, E>` → the union of both. Runtime `StatusCode` values are invisible to any type system, so a truthful spec requires them in the signature.

**4 · schemars 1.x.** JSON Schema 2020-12 *is* the OpenAPI 3.1 dialect — one derive feeds both the type and the document, with named schemas registered under `components/schemas`.

## Status — v0 scaffold

Honest about what exists today.

**Works:** `OasRouter` mirroring `Router::route`, method routers `get`/`post`/`put`/`delete` (top-level and chained), request bodies from `Json<T>`, query parameters inlined per field from `Query<T>`, path parameters named from the route template, no-op inputs (`State`, `Extension`, `HeaderMap`, `Option<T>`), typed responses, `into_router()` serving `/openapi.json`, and the compile-fail guarantee (covered by `trybuild` snapshots).

**Not yet:** doc comments as summaries/descriptions (that is what the reserved `axum-oas-macros` crate is for — one *inert* attribute whose only job is text traits cannot see), per-field path parameter schemas (v0 types them as `string`), header/cookie parameters, form and multipart bodies, `nest`/`merge` spec composition, security schemes, request validation (schemas are emitted; enforcement is future work), Swagger-UI/Redoc serving.

**Requires:** Rust 1.85+ (edition 2024), axum 0.8, schemars 1.x.

## Compared to what exists

| | axum-oas | [utoipa](https://github.com/juhaku/utoipa) | [aide](https://github.com/tamasfe/aide) |
|---|---|---|---|
| **Handler described** | from its types, at registration | re-described in `#[utoipa::path]` attributes | from its types, at registration |
| **Undescribable handler** | **compile error** | compiles; spec omits what you didn't annotate | compiles; silently undocumented |
| **Status codes / descriptions** | in the return type | in attributes | in `*_with` transform closures |

utoipa is framework-agnostic and therefore token-level: it cannot read a handler it does not annotate. aide shares axum-oas's mechanism and is more complete today; axum-oas differs on the property that matters most to us — it refuses to produce a spec that quietly disagrees with the code.

## License

MIT
