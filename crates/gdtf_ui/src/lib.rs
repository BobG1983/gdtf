//! Hand-rolled `bevy_ui` layer for GDTF.
//!
//! This crate is the seam the menu / HUD work hangs on. It owns no combat rules
//! (those live in `gdtf_battle_sim`) and deliberately depends on **bevy only**,
//! so `gdtf_app` can depend on it without forming a dependency cycle.
//!
//! Today it is a compile-and-wire skeleton: [`UiPlugin`] registers cleanly into
//! the app but installs nothing yet. Later tickets attach the UI systems,
//! resources, and assets to [`UiPlugin::build`].
//!
//! The data-driven [`theme`] module defines the on-disk theme schema, the runtime
//! [`GdtfTheme`](theme::GdtfTheme) resource, and the pure spec-to-resource
//! resolution; population of that resource lands with later tickets.

pub mod theme;

use bevy::prelude::*;

/// The GDTF UI plugin — the single registration seam for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the seam in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// [`build`](UiPlugin::build) is intentionally empty for now: the skeleton
/// proves the registration path end-to-end (see the `gdtf_app` headless harness
/// test asserting `is_plugin_added::<UiPlugin>()`) before any UI systems exist.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Empty build seam. Later tickets add the UI systems/resources here; for
    /// now installing nothing keeps the skeleton bevy-only and side-effect-free.
    fn build(&self, _app: &mut App) {}
}
