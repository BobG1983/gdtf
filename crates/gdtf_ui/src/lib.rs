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
//!
//! The [`widgets`] module owns the reusable spawn helpers
//! ([`spawn_panel`](widgets::spawn_panel) / [`spawn_button`](widgets::spawn_button)),
//! the [`DisabledButton`](widgets::DisabledButton) marker, and the disabled-dim
//! pass; the [`interaction`] module owns the theme-derived hover/press feedback
//! system. Both compose *on top of* [`apply_theme`](themed::apply_theme)'s base
//! look, ordered after it.

pub mod focus_nav;
pub mod interaction;
pub mod theme;
pub mod themed;
pub mod widgets;

use bevy::prelude::*;
pub use interaction::{sync_hover_to_focus, theme_interaction};
pub use widgets::{
    ButtonLabel, DimFactor, DisabledButton, dim_disabled_buttons, dimmed_fill, spawn_button,
    spawn_panel,
};

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
/// ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)), the central theming pass
/// ([`apply_theme`](themed::apply_theme)), the GTW-118 widget interaction
/// layer ([`theme_interaction`](interaction::theme_interaction) +
/// [`dim_disabled_buttons`](widgets::dim_disabled_buttons)), and the GTW-141
/// mouse hover→focus bridge ([`sync_hover_to_focus`](interaction::sync_hover_to_focus)).
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin, the central theming system, and the
    /// theme-derived widget interaction layer.
    ///
    /// [`apply_theme`](themed::apply_theme) runs in [`Update`] inside the named
    /// [`UiSystems::ApplyTheme`](themed::UiSystems::ApplyTheme) set, gated by
    /// `.run_if(resource_exists::<GdtfTheme>)` so it is inert until the theme is
    /// populated (pre-`Load`) and never panics on its absence (bevy-traps rule
    /// 1). The named set is the deterministic ordering anchor the later retheme
    /// trigger (GTW-137) and the interaction-feedback systems (GTW-118) order
    /// against (bevy-traps rule 3).
    ///
    /// The GTW-118 interaction layer —
    /// [`theme_interaction`](interaction::theme_interaction) (hover/press swap)
    /// and [`dim_disabled_buttons`](widgets::dim_disabled_buttons) (disabled
    /// dim) — runs in [`Update`] `.after(`[`UiSystems::ApplyTheme`](themed::UiSystems::ApplyTheme)`)`
    /// so each composes on top of the freshest base look (bevy-traps rule 3);
    /// both guard the theme internally (`Option<Res<GdtfTheme>>`), so they too
    /// are inert before the resource is populated (bevy-traps rule 1).
    ///
    /// The GTW-141 mouse hover→focus bridge
    /// ([`sync_hover_to_focus`](interaction::sync_hover_to_focus)) runs in the
    /// **same** `.after(UiSystems::ApplyTheme)` band: it reads the
    /// [`Interaction`](bevy::ui::Interaction) that `bevy_ui`'s built-in
    /// `ui_focus_system` already wrote from the mouse in `PreUpdate` and moves
    /// [`InputFocus`](bevy::input_focus::InputFocus) onto the hovered button, so
    /// pointer hover and keyboard / gamepad navigation share one focus cursor.
    /// No extra picking plugin is added — `ui_focus_system` (in `DefaultPlugins`)
    /// already drives [`Interaction`](bevy::ui::Interaction), so real clicks
    /// reach GTW-122's mouse action layer with no new wiring (bevy-traps).
    ///
    /// Later tickets hang further UI systems/resources here; the layer stays
    /// bevy-only (no ecosystem crate).
    fn build(&self, app: &mut App) {
        app.add_plugins(FocusNavPlugin).add_systems(
            Update,
            (
                apply_theme
                    .in_set(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<GdtfTheme>),
                (theme_interaction, dim_disabled_buttons, sync_hover_to_focus)
                    .after(UiSystems::ApplyTheme),
            ),
        );
    }
}
