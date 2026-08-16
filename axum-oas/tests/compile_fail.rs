//! Pillar 2: compile-error over silent omission.
//!
//! These trybuild cases prove that a handler axum-oas cannot fully describe
//! is rejected *at compile time* with the curated
//! `#[diagnostic::on_unimplemented]` message — the exact situation in which
//! other libraries silently emit an under-documented spec.

#[test]
fn undescribable_handlers_do_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
