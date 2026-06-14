//! Hand-rolled `bevy_ui` layer for GDTF.
//!
//! This crate is the seam the menu / HUD work hangs on. It owns no combat rules
//! (those live in `gdtf_battle_sim`) and deliberately depends on **bevy only**,
//! so `gdtf_app` can depend on it without forming a dependency cycle.
//!
//! [`UiPlugin`] is the single registration seam: today it installs the
//! [`focus_nav`] sub-plugin ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)) and
//! nothing else. Later tickets attach further UI systems, resources, and assets
//! to [`UiPlugin::build`].
//!
//! The [`focus_nav`] module wires Bevy's `input_focus` framework and bridges
//! keyboard + gamepad input onto directional focus navigation; see its docs for
//! the activation-message decision.
//!
//! The data-driven [`theme`] module defines the on-disk theme schema, the runtime
//! [`GdtfTheme`](theme::GdtfTheme) resource, and the pure spec-to-resource
//! resolution; population of that resource lands with later tickets.

pub mod focus_nav;
pub mod theme;

use bevy::prelude::*;

use crate::focus_nav::FocusNavPlugin;

/// The GDTF UI plugin — the single registration seam for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the seam in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// It currently installs the focus-navigation layer
/// ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)) and nothing else.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin. Later tickets hang further UI
    /// systems/resources here; the layer stays bevy-only (no ecosystem crate).
    fn build(&self, app: &mut App) {
        app.add_plugins(FocusNavPlugin);
    }
}
