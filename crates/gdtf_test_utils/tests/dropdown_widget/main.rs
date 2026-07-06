//! GTW-410 headless integration test for the [`Dropdown`](gdtf_ui::Dropdown) widget, driving
//! the REAL widget on the REAL `bevy_ui` layout path (verification rule 3).
//!
//! These run on the [`GdtfUiTestAppBuilder`] `DefaultPlugins` headless harness (real layout
//! geometry — a computed [`ComputedNode`] — and a real camera) with [`gdtf_ui::UiPlugin`] and
//! `register_dropdown::<TestChoice>` added, so the dropdown's open / select / dismiss drivers
//! actually run. Each test is pin-discriminating: it would fail if the z-order, the selection
//! message, the shown-label mutation, or the dismiss-without-change broke.

mod harness;
mod highlight;
mod keyboard;
mod mouse;
