//! The [`UiPlugin`] registration seam and its theme-asset message-buffer gate.

use bevy::prelude::*;
use gdtf_assets::RonAsset;

use crate::{
    focus_nav::FocusNavPlugin,
    interaction::{sync_hover_to_focus, theme_interaction},
    retheme::redrive_theme_on_asset_event,
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{UiSystems, any_themed_added, apply_theme},
    widgets::{paint_active_buttons, paint_disabled_buttons},
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
/// ([`FocusNavPlugin`](crate::focus_nav::FocusNavPlugin)), the central theming pass
/// ([`apply_theme`](crate::themed::apply_theme)), the GTW-137 live-retheme trigger
/// ([`redrive_theme_on_asset_event`](crate::retheme::redrive_theme_on_asset_event)),
/// the GTW-118 widget interaction layer
/// ([`theme_interaction`](crate::interaction::theme_interaction) +
/// [`paint_disabled_buttons`](crate::widgets::paint_disabled_buttons)), and the
/// GTW-141 mouse hover→focus bridge
/// ([`sync_hover_to_focus`](crate::interaction::sync_hover_to_focus)).
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin, the central theming system, and the
    /// theme-derived widget interaction layer.
    ///
    /// [`apply_theme`](crate::themed::apply_theme) runs in [`Update`] inside the named
    /// [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme) set. It is
    /// **change-driven** (GTW-144): gated by
    /// `resource_exists::<GdtfTheme>().and(resource_changed::<GdtfTheme>.or(`[`any_themed_added`](crate::themed::any_themed_added)`))`,
    /// so it runs only when the theme changed (the `Load` insert, or the GTW-137
    /// re-derive — repainting ALL [`Themed`](crate::themed::Themed) entities = the
    /// retheme) or a new [`Themed`](crate::themed::Themed) entity appeared (so a
    /// freshly-spawned widget still gets its base look), and **never** on a
    /// steady-state frame — where re-running every frame would clobber the GTW-118
    /// hover/press feedback. The `resource_exists` arm also keeps it inert and
    /// panic-free before the theme is populated (bevy-traps rule 1). The named set
    /// is the deterministic ordering anchor (bevy-traps rule 3).
    ///
    /// The GTW-137 live-retheme trigger
    /// ([`redrive_theme_on_asset_event`](crate::retheme::redrive_theme_on_asset_event))
    /// runs in [`Update`] `.before(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)`
    /// so a re-derived theme repaints the same frame (bevy-traps rule 3). It
    /// reads the [`AssetEvent`](bevy::asset::AssetEvent) **message** stream
    /// (bevy-traps rule 4) and is gated on both `GdtfTheme` and the theme
    /// asset-event message buffer existing — the buffer is registered only with
    /// `AssetPlugin` + the GTW-136 loader, so the guard keeps its `MessageReader`
    /// from failing param validation under an asset-less `MinimalPlugins` harness
    /// (bevy-traps rule 1).
    ///
    /// The GTW-118 interaction layer —
    /// [`theme_interaction`](crate::interaction::theme_interaction) (hover/press swap),
    /// [`paint_disabled_buttons`](crate::widgets::paint_disabled_buttons) (disabled
    /// fill), and the GTW-253
    /// [`paint_active_buttons`](crate::widgets::paint_active_buttons) (active / toggled-on
    /// fill, skipping disabled buttons) — runs in [`Update`] `.after(`[`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)`)`
    /// so each composes on top of the freshest base look (bevy-traps rule 3);
    /// all guard the theme internally (`Option<Res<GdtfTheme>>`), so they too
    /// are inert before the resource is populated (bevy-traps rule 1).
    ///
    /// The GTW-141 mouse hover→focus bridge
    /// ([`sync_hover_to_focus`](crate::interaction::sync_hover_to_focus)) runs in the
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
                // GTW-118 disabled paint, GTW-253 active paint, and the hover/press
                // swap all compose ON TOP of the base look, ordered after the ApplyTheme
                // set (bevy-traps rule 3). `paint_active_buttons` overrides an active
                // (toggled-on) button's fill with the theme's `active` color; it skips
                // `DisabledButton` (`Without<DisabledButton>`), so disabled+active
                // resolves to disabled.
                //
                // GTW-266 — active is STICKY: `theme_interaction` now EXCLUDES
                // `ActiveButton` (`Without<ActiveButton>`), so the two no longer write the
                // same active button's `BackgroundColor` the same frame. As belt-and-braces
                // (and so a future widget that drops that exclusion still resolves
                // active-wins deterministically), `paint_active_buttons` is ALSO ordered
                // `.after(theme_interaction)` — the active fill is the last writer, never a
                // flickering race (bevy-traps rule 3).
                (
                    theme_interaction,
                    paint_disabled_buttons,
                    paint_active_buttons.after(theme_interaction),
                    sync_hover_to_focus,
                )
                    .after(UiSystems::ApplyTheme),
            ),
        );
    }
}
