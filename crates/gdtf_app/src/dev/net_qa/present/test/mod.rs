//! Tests for the GTW-764 offscreen-capture present path.
//!
//! - [`harness`] — the shared headless `DefaultPlugins` app with a real primary window, no GPU
//!   backend, and no winit event loop.
//! - [`present`] — the [`CapturePresentPlugin`](super::CapturePresentPlugin) wiring: the
//!   `WinitSettings` override, the offscreen target's size / format / `COPY_SRC`, the world +
//!   UI camera retarget, and the present camera targeting the window.
//! - [`capture_source`] — GTW-764 C4: the T7 pump captures the offscreen image when the target
//!   exists and falls back to the window otherwise.

mod capture_source;
mod harness;
mod present;
