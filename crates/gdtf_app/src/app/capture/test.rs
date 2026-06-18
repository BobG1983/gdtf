//! Unit tests for the DEV-ONLY screenshot / visual-QA config logic (GTW-297).
//!
//! These cover ONLY the `from_env` config logic: the [`capture_path`] gate (disabled
//! when `GDTF_CAPTURE_PATH` is unset / empty) and the [`CaptureFrame`] default + parse.
//! The ACTUAL screenshot capture needs a real render device, so it is NOT headless-
//! testable — it is verified by RUNNING the app (the orchestrator does so, then `Read`s
//! the PNG). See the `plugin` module doc for the invocation.
//!
//! To stay deterministic under parallel tests, these drive the REAL pure parse cores
//! ([`CaptureFrame::parse`] / [`parse_capture_path`]) with INJECTED values rather than
//! mutating the process-global env vars — the same code `from_env` / `capture_path` run.

use super::DevCapturePlugin;
// `CaptureFrame` + the pure parse cores are not re-exported from `mod.rs` (only
// `DevCapturePlugin` is, for the binary), so reach them through the `plugin` submodule.
use super::plugin::{CaptureFrame, capture_path, parse_capture_path};

/// `CaptureFrame::DEFAULT` is 15 frames and `Default` agrees with it — the wait the
/// affordance uses when `GDTF_CAPTURE_FRAME` is unset, so the UI layout flushes before
/// the capture.
#[test]
fn capture_frame_default_is_fifteen() {
    assert_eq!(*CaptureFrame::DEFAULT, 15);
    assert_eq!(CaptureFrame::default(), CaptureFrame::DEFAULT);
}

/// The REAL frame parse ([`CaptureFrame::parse`], the core of `from_env`) takes a valid
/// `u32` and falls back to [`CaptureFrame::DEFAULT`] for an empty / non-numeric / absent
/// value — proving the parse path is wired without mutating the shared environment.
#[test]
fn capture_frame_parses_or_defaults() {
    assert_eq!(*CaptureFrame::parse(Some("0")), 0);
    assert_eq!(*CaptureFrame::parse(Some("7")), 7);
    assert_eq!(*CaptureFrame::parse(Some(" 42 ")), 42);
    for bad in ["", "  ", "x", "-1", "1.5", "12abc"] {
        assert_eq!(
            CaptureFrame::parse(Some(bad)),
            CaptureFrame::DEFAULT,
            "{bad:?} should fall back to the default frame",
        );
    }
    assert_eq!(CaptureFrame::parse(None), CaptureFrame::DEFAULT);
}

/// The REAL capture-path gate ([`parse_capture_path`], the core of `capture_path`) is
/// `None` for an unset / empty / whitespace value and `Some` for a real path — the gate
/// the affordance keys on. Driving the injected core avoids racing the process-global
/// env var across parallel tests.
#[test]
fn capture_path_gate_rejects_empty() {
    assert!(
        parse_capture_path(None).is_none(),
        "unset path leaves the affordance inert",
    );
    assert!(
        parse_capture_path(Some("")).is_none(),
        "empty path leaves it inert",
    );
    assert!(
        parse_capture_path(Some("   ")).is_none(),
        "whitespace path leaves it inert",
    );
    assert_eq!(
        parse_capture_path(Some("/abs/out.png")),
        Some(std::path::PathBuf::from("/abs/out.png")),
        "a real path enables the affordance",
    );
}

/// `DevCapturePlugin::from_env` defers to the env-var gate: it is enabled exactly when
/// [`capture_path`] returns a path, and configures a [`CaptureFrame`] iff enabled.
/// Reading via the real `from_env` path keeps the assertion honest without injecting or
/// mutating state — whatever the ambient `GDTF_CAPTURE_PATH` is, the plugin's
/// enabled-ness must match the gate.
///
/// `enabled` / `capture_frame` are `#[cfg(test)]` inherent surface (absent from the
/// binary build, keeping it `dead_code`-clean), so this test reaches them directly.
#[test]
fn from_env_enabled_matches_the_gate() {
    let plugin = DevCapturePlugin::from_env();
    assert_eq!(
        plugin.enabled(),
        capture_path().is_some(),
        "from_env must activate exactly when GDTF_CAPTURE_PATH is set",
    );
    // When enabled, a CaptureFrame is configured; when inert, none is.
    assert_eq!(plugin.enabled(), plugin.capture_frame().is_some());
}
