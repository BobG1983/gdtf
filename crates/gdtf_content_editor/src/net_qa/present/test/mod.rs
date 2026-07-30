//! Tests for the GTW-918 editor offscreen-capture present path.
//!
//! - [`harness`] — the shared headless `DefaultPlugins` app with a real primary window (at a
//!   non-`1.0` scale factor), no GPU backend and no winit event loop, plus the two editor-shaped
//!   fixtures: an egui camera spawned window-targeted, and the `bevy_egui` input-map entry the
//!   retarget gates on.
//! - [`present`] — the plugin wiring: the `WinitSettings` override, the target's size / format /
//!   `COPY_SRC`, the capture source naming that target, and the `Update` schedule placement.
//! - [`retarget`] — the retarget itself: the input-mapping gate, the whole-target value the
//!   camera is aimed at, the window's scale factor, and the primary egui context staying on the
//!   retargeted camera.
//! - [`blit`] — the present pass: its order, its render layer, its blit sprite, and the gate that
//!   keeps it from compositing over the window before the retarget has fired (GTW-922).
//! - [`inert`] — an env-gated-off editor wires none of it.

mod blit;
mod harness;
mod inert;
mod present;
mod retarget;
