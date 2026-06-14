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
//!
//! The [`themed`] module owns the [`Themed`](themed::Themed) marker and the
//! central [`apply_theme`](themed::apply_theme) system — the hot-reload seam that
//! paints theme-derived visuals onto themed entities from the live
//! [`GdtfTheme`](theme::GdtfTheme).

pub mod focus_nav;
pub mod theme;
pub mod themed;

use bevy::prelude::*;

use crate::{
    focus_nav::FocusNavPlugin,
    theme::GdtfTheme,
    themed::{UiSystems, apply_theme},
};

/// The GDTF UI plugin — the single registration seam for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the seam in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// It currently installs the focus-navigation layer
/// ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)) and the central theming pass
/// ([`apply_theme`](themed::apply_theme)).
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin and the central theming system.
    ///
    /// [`apply_theme`](themed::apply_theme) runs in [`Update`] inside the named
    /// [`UiSystems::ApplyTheme`](themed::UiSystems::ApplyTheme) set, gated by
    /// `.run_if(resource_exists::<GdtfTheme>)` so it is inert until the theme is
    /// populated (pre-`Load`) and never panics on its absence (bevy-traps rule
    /// 1). The named set is the deterministic ordering anchor the later retheme
    /// trigger (GTW-137) and interaction-feedback systems (GTW-118) order against
    /// (bevy-traps rule 3). Later tickets hang further UI systems/resources here;
    /// the layer stays bevy-only (no ecosystem crate).
    fn build(&self, app: &mut App) {
        app.add_plugins(FocusNavPlugin).add_systems(
            Update,
            apply_theme
                .in_set(UiSystems::ApplyTheme)
                .run_if(resource_exists::<GdtfTheme>),
        );
    }
}
