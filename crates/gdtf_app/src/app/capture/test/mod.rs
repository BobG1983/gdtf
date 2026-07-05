//! Unit tests for the DEV-ONLY screenshot / fire-trigger config + the headless
//! fire-trigger path (GTW-297, extended GTW-306).
//!
//! The config tests cover the pure parse cores: the [`capture_path`] gate, the
//! [`CaptureFrame`] / [`CaptureFrames`] / [`FireAtFrame`] parse, and the multi-frame
//! [`frame_path`] naming. They drive the REAL pure cores with INJECTED values rather than
//! mutating the process-global env vars (deterministic under parallel tests) — the same
//! code `from_env` / `capture_path` run.
//!
//! The ACTUAL screenshot capture needs a real render device, so it is NOT headless-
//! testable — it is verified by RUNNING the app. The fire trigger, by contrast, IS
//! headless: it only writes a `FireRequested` message, so the
//! [`trigger_fire_at_frame_emits_on_the_real_path`] test drives the REAL system on a
//! minimal app and asserts exactly one `FireRequested` at frame N.

mod config;
mod fall;
mod fire;
