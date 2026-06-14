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
//! The [`retheme`] module owns the live-reapply logic
//! ([`redrive_theme_on_asset_event`](retheme::redrive_theme_on_asset_event)): on
//! an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for the active theme
//! asset it re-derives [`GdtfTheme`](theme::GdtfTheme) in place, and the
//! change-driven [`apply_theme`](themed::apply_theme) repaints every
//! [`Themed`](themed::Themed) entity the same frame — no restart (GTW-137).
//!
//! The [`widgets`] module owns the reusable spawn helpers
//! ([`spawn_panel`](widgets::spawn_panel) / [`spawn_button`](widgets::spawn_button)),
//! the [`DisabledButton`](widgets::DisabledButton) marker, and the disabled-dim
//! pass; the [`interaction`] module owns the theme-derived hover/press feedback
//! system. Both compose *on top of* [`apply_theme`](themed::apply_theme)'s base
//! look, ordered after it.

pub mod focus_nav;
pub mod interaction;
pub mod retheme;
pub mod theme;
pub mod themed;
pub mod widgets;

use bevy::prelude::*;
use gdtf_assets::RonAsset;
pub use interaction::{sync_hover_to_focus, theme_interaction};
pub use retheme::redrive_theme_on_asset_event;
pub use themed::any_themed_added;
pub use widgets::{
    ButtonLabel, DimFactor, DisabledButton, dim_disabled_buttons, dimmed_fill, spawn_button,
    spawn_panel,
};

use crate::{
    focus_nav::FocusNavPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, apply_theme},
};

/// The message buffer the GTW-137 retheme system reads: asset events for the
/// theme `RonAsset`.
///
/// Named here so the [`UiPlugin`] run condition can gate the retheme system on its
/// existence — the buffer is only registered when `AssetPlugin` (and the GTW-136
/// `init_ron_asset::<GdtfThemeSpec>()`) is present, so gating on it keeps the
/// system inert under a `MinimalPlugins` harness with no asset support (where its
/// `MessageReader` would otherwise fail param validation), without weakening the
/// production path (bevy-traps rule 1).
type ThemeAssetMessages = Messages<AssetEvent<RonAsset<GdtfThemeSpec>>>;

/// The GDTF UI plugin — the single registration seam for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the seam in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// It currently installs the focus-navigation layer
/// ([`FocusNavPlugin`](focus_nav::FocusNavPlugin)), the central theming pass
/// ([`apply_theme`](themed::apply_theme)), the GTW-137 live-retheme trigger
/// ([`redrive_theme_on_asset_event`](retheme::redrive_theme_on_asset_event)), the
/// GTW-118 widget interaction layer
/// ([`theme_interaction`](interaction::theme_interaction) +
/// [`dim_disabled_buttons`](widgets::dim_disabled_buttons)), and the GTW-141
/// mouse hover→focus bridge ([`sync_hover_to_focus`](interaction::sync_hover_to_focus)).
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin, the central theming system, and the
    /// theme-derived widget interaction layer.
    ///
    /// [`apply_theme`](themed::apply_theme) runs in [`Update`] inside the named
    /// [`UiSystems::ApplyTheme`](themed::UiSystems::ApplyTheme) set. It is
    /// **change-driven** (GTW-144): gated by
    /// `resource_exists::<GdtfTheme>().and(resource_changed::<GdtfTheme>.or(`[`any_themed_added`](themed::any_themed_added)`))`,
    /// so it runs only when the theme changed (the `Load` insert, or the GTW-137
    /// re-derive — repainting ALL [`Themed`](themed::Themed) entities = the
    /// retheme) or a new [`Themed`](themed::Themed) entity appeared (so a
    /// freshly-spawned widget still gets its base look), and **never** on a
    /// steady-state frame — where re-running every frame would clobber the GTW-118
    /// hover/press feedback. The `resource_exists` arm also keeps it inert and
    /// panic-free before the theme is populated (bevy-traps rule 1). The named set
    /// is the deterministic ordering anchor (bevy-traps rule 3).
    ///
    /// The GTW-137 live-retheme trigger
    /// ([`redrive_theme_on_asset_event`](retheme::redrive_theme_on_asset_event))
    /// runs in [`Update`] `.before(`[`UiSystems::ApplyTheme`](themed::UiSystems::ApplyTheme)`)`
    /// so a re-derived theme repaints the same frame (bevy-traps rule 3). It
    /// reads the [`AssetEvent`](bevy::asset::AssetEvent) **message** stream
    /// (bevy-traps rule 4) and is gated on both `GdtfTheme` and the theme
    /// asset-event message buffer existing — the buffer is registered only with
    /// `AssetPlugin` + the GTW-136 loader, so the guard keeps its `MessageReader`
    /// from failing param validation under an asset-less `MinimalPlugins` harness
    /// (bevy-traps rule 1).
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
                // GTW-137: on a theme-asset `AssetEvent::Modified` for the active
                // handle, re-derive and overwrite `GdtfTheme`. Ordered BEFORE the
                // ApplyTheme set so the resulting theme-changed marks repaint the
                // same frame (bevy-traps rule 3). Gated on `GdtfTheme` existing AND
                // the asset-event message buffer existing: the latter is only
                // registered with `AssetPlugin` + `init_ron_asset`, so this keeps
                // the system's `MessageReader` from failing param validation under
                // a `MinimalPlugins` harness with no asset support (bevy-traps rule
                // 1).
                redrive_theme_on_asset_event
                    .before(UiSystems::ApplyTheme)
                    .run_if(
                        resource_exists::<GdtfTheme>.and(resource_exists::<ThemeAssetMessages>),
                    ),
                // GTW-144: `apply_theme` is CHANGE-DRIVEN — it runs only when the
                // theme changed (the Load insert OR a GTW-137 re-derive: repaints
                // ALL Themed = the retheme) OR a new `Themed` entity appeared (so
                // freshly-spawned widgets still get their base look). On steady
                // frames it does not run, so it never clobbers the GTW-118
                // hover/press feedback that only updates on `Changed<Interaction>`.
                apply_theme.in_set(UiSystems::ApplyTheme).run_if(
                    resource_exists::<GdtfTheme>
                        .and(resource_changed::<GdtfTheme>.or(any_themed_added)),
                ),
                (theme_interaction, dim_disabled_buttons, sync_hover_to_focus)
                    .after(UiSystems::ApplyTheme),
            ),
        );
    }
}
