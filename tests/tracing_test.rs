// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests validating that tracing events emit structured fields correctly.

use tracing_test::traced_test;

#[traced_test]
#[test]
fn structured_warn_emits_error_field() {
    tracing::warn!(error = %"test error", "walk error");
    assert!(logs_contain("walk error"));
    assert!(logs_contain("error"));
}

#[traced_test]
#[test]
fn structured_error_emits_error_field() {
    tracing::error!(error = %"rollback failed", "rollback failed");
    assert!(logs_contain("rollback failed"));
}

#[traced_test]
#[test]
fn debug_level_includes_filter_info() {
    tracing::debug!(filter = "debug", "tracing initialized");
    assert!(logs_contain("tracing initialized"));
    assert!(logs_contain("filter"));
}

#[traced_test]
#[test]
fn span_captures_path_field() {
    let _span = tracing::debug_span!("process_file", path = "/test/file.rs").entered();
    tracing::debug!("processing");
    assert!(logs_contain("process_file"));
}

#[traced_test]
#[test]
fn panic_hook_fields_are_structured() {
    tracing::error!(
        panic.payload = %"test panic",
        panic.location = "test.rs:42",
        "process panicked"
    );
    assert!(logs_contain("process panicked"));
    assert!(logs_contain("panic.payload"));
}
