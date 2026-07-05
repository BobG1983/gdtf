//! GTW-250 pan navigation (mouse-edge + keyboard + gamepad right stick) and the GTW-259
//! gamepad-cursor edge-pan: the view tunables, the pure direction/velocity helpers, the
//! presenter-defined cursor message, and the two pan systems.
//!
//! GTW-271 reworked the mouse / gamepad-cursor edge-pan to key off the MAP VIEWPORT rect
//! ([`Camera::logical_viewport_rect`](bevy::prelude::Camera::logical_viewport_rect)) rather
//! than the whole window: the cursor only edge-pans
//! when it sits INSIDE the world camera's viewport sub-rect, so a cursor in a UI margin (over a
//! panel) never pans the map. That viewport-inside gate REPLACED the GTW-262
//! `Interaction`-over-UI suppression (which caused the "green bar" and over-suppressed
//! keyboard / stick panning), so this module no longer reads `bevy_ui::Interaction`.

mod dir;
mod gamepad_cursor;
mod pan_camera;
mod tunables;

pub use dir::{keyboard_pan_dir, mouse_edge_dir, pan_velocity, stick_pan_dir, viewport_edge_dir};
pub use gamepad_cursor::{GamepadCursorMoved, pan_camera_on_gamepad_cursor_edge};
pub use pan_camera::pan_camera;
pub use tunables::{EdgeBandPx, PanSpeed, STICK_DEADZONE, StickDeadzone};
