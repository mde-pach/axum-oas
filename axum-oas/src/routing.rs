//! Type capture at route registration.
//!
//! This is the pillar-1 machinery: the wrapper method routers
//! [`get`]/[`post`]/[`put`]/[`delete`] accept any real `axum` handler and,
//! *at the same call site*, capture a `fn` pointer that can later describe
//! the handler's extractors and return type into an OpenAPI operation.
//!
//! The capture works by mirroring axum's own `Handler` blanket impls with the
//! [`OasHandler`] trait: axum implements `Handler<((),), S>` for zero-argument
//! async fns and `Handler<(M, T1, ..., Tn), S>` for fns taking extractors, so
//! `OasHandler` provides impls of exactly the same shape. The handler's
//! concrete return type — which axum erases — is recovered through the
//! `F: FnOnce(...) -> Fut` bound (`Fut::Output: OperationOutput`).

use std::future::Future;

use axum::handler::Handler;
use axum::routing::MethodRouter;
use schemars::generate::SchemaGenerator;

use crate::operation::{OperationInput, OperationOutput};
use crate::spec::Operation;

/// A monomorphized description function for one handler.
pub type DescribeFn = fn(&mut Operation, &mut SchemaGenerator);

/// The describability side of [`axum::handler::Handler`].
///
/// `H: Handler<T, S> + OasHandler<T, S>` is the full bound used by the
/// wrapper method routers: axum checks that the function *runs*, axum-oas
/// checks that it is *describable* — over the same type tuple `T`, so type
/// inference resolves both from one call site.
pub trait OasHandler<T, S> {
    /// Describe this handler's inputs and output into `operation`.
    fn describe(operation: &mut Operation, generator: &mut SchemaGenerator);
}

// Zero-argument handlers: axum's `Handler<((),), S>` impl shape.
impl<F, Fut, Res, S> OasHandler<((),), S> for F
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Res>,
    Res: OperationOutput,
{
    fn describe(operation: &mut Operation, generator: &mut SchemaGenerator) {
        Res::operation_output(operation, generator);
    }
}

// Handlers with extractors: axum's `Handler<(M, T1, ..., Tn), S>` impl shape,
// where `M` is axum's private `FromRequest`-vs-`FromRequestParts` marker.
macro_rules! impl_oas_handler {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case)]
        impl<F, Fut, Res, M, S, $($ty,)*> OasHandler<(M, $($ty,)*), S> for F
        where
            F: FnOnce($($ty,)*) -> Fut,
            Fut: Future<Output = Res>,
            Res: OperationOutput,
            $( $ty: OperationInput, )*
        {
            fn describe(operation: &mut Operation, generator: &mut SchemaGenerator) {
                $( $ty::operation_input(operation, generator); )*
                Res::operation_output(operation, generator);
            }
        }
    };
}

impl_oas_handler!(T1);
impl_oas_handler!(T1, T2);
impl_oas_handler!(T1, T2, T3);
impl_oas_handler!(T1, T2, T3, T4);
impl_oas_handler!(T1, T2, T3, T4, T5);
impl_oas_handler!(T1, T2, T3, T4, T5, T6);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_oas_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_oas_handler!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_oas_handler!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

/// An [`axum::routing::MethodRouter`] plus the captured description functions
/// of every handler registered on it.
pub struct OasMethodRouter<S = ()> {
    pub(crate) inner: MethodRouter<S>,
    pub(crate) operations: Vec<(&'static str, DescribeFn)>,
}

impl<S> std::fmt::Debug for OasMethodRouter<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OasMethodRouter")
            .field(
                "methods",
                &self.operations.iter().map(|(m, _)| *m).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

macro_rules! top_level_method {
    ( $name:ident ) => {
        #[doc = concat!(
                    "Route `", stringify!($name), "` requests to the given handler, \
             capturing its extractor and response types for the OpenAPI \
             document.\n\nDrop-in replacement for [`axum::routing::",
                    stringify!($name), "`], with the additional requirement that the \
            handler is fully describable (every extractor implements \
            [`OperationInput`] and the return type implements \
            [`OperationOutput`])."
                )]
        pub fn $name<H, T, S>(handler: H) -> OasMethodRouter<S>
        where
            H: Handler<T, S> + OasHandler<T, S>,
            T: 'static,
            S: Clone + Send + Sync + 'static,
        {
            OasMethodRouter {
                inner: axum::routing::$name(handler),
                operations: vec![(stringify!($name), <H as OasHandler<T, S>>::describe)],
            }
        }
    };
}

top_level_method!(get);
top_level_method!(post);
top_level_method!(put);
top_level_method!(delete);

macro_rules! chained_method {
    ( $name:ident ) => {
        #[doc = concat!(
                    "Chain an additional `", stringify!($name),
                    "` handler onto this method router (like `axum`'s \
             `MethodRouter::", stringify!($name), "`)."
                )]
        pub fn $name<H, T>(mut self, handler: H) -> Self
        where
            H: Handler<T, S> + OasHandler<T, S>,
            T: 'static,
        {
            self.inner = self.inner.$name(handler);
            self.operations
                .push((stringify!($name), <H as OasHandler<T, S>>::describe));
            self
        }
    };
}

impl<S> OasMethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    chained_method!(get);
    chained_method!(post);
    chained_method!(put);
    chained_method!(delete);
}
