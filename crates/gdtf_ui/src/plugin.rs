//! The [`UiPlugin`] registration point and its theme-asset message-buffer gate.

use bevy::prelude::*;
use gdtf_assets::{RonAsset, redrive_hot_ron_resource};

use crate::{
    focus_nav::FocusNavPlugin,
    theming::{
        retheme::theme_hot_ron_chain,
        theme::{GdtfTheme, GdtfThemeSpec},
        themed::{UiSystems, any_themed_added, apply_theme},
    },
    widgets::{
        core::{
            SegmentSelected, ToggleFlipped, drive_switches, paint_active_buttons,
            paint_disabled_buttons, repaint_segments, select_segment_on_press,
        },
        interaction::{
            repaint_deactivated_buttons, repaint_theme_change, sync_hover_to_focus,
            theme_interaction,
        },
    },
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

/// The GDTF UI plugin — the single registration point for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the plugin in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// It currently installs the focus-navigation layer
/// ([`FocusNavPlugin`](crate::focus_nav::FocusNavPlugin)), the central theming pass
/// ([`apply_theme`](crate::theming::themed::apply_theme)), the GTW-137 live-retheme trigger
/// (the GTW-564 generic [`redrive_hot_ron_resource`]`::<GdtfThemeSpec, GdtfTheme>`,
/// configured by [`theme_hot_ron_chain`](crate::theming::retheme::theme_hot_ron_chain)),
/// the GTW-118 widget interaction layer
/// ([`theme_interaction`](crate::widgets::interaction::theme_interaction) +
/// [`paint_disabled_buttons`](crate::widgets::core::paint_disabled_buttons)), and the
/// GTW-141 mouse hover→focus bridge
/// ([`sync_hover_to_focus`](crate::widgets::interaction::sync_hover_to_focus)).
///
/// (The GTW-410 `Dropdown<T>` combobox — with its type-agnostic message + `Escape` emitter +
/// popup positioner this plugin used to install, plus the per-option-id `register_dropdown::<T>`
/// caller registration function — the GTW-411 `TextField`/`NumericField` editable fields, the GTW-412
/// `ScrollList` — with its guarded `ScrollAreaPlugin` / `ScrollbarPlugin` adds — and the
/// GTW-416 `Accordion` driver were RETIRED by GTW-655/GTW-636: their only consumers, the
/// GTW-434 procgen visualizer and the in-game gang editor, moved off / were retired.)
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// Adds the focus-navigation sub-plugin, the central theming system, and the
    /// theme-derived widget interaction layer.
    ///
    /// [`apply_theme`](crate::theming::themed::apply_theme) runs in [`Update`] inside the named
    /// [`UiSystems::ApplyTheme`](crate::theming::themed::UiSystems::ApplyTheme) set. It is
    /// **change-driven** (GTW-144): gated by
    /// `resource_exists::<GdtfTheme>().and_then(resource_changed::<GdtfTheme>.or_else(`[`any_themed_added`](crate::theming::themed::any_themed_added)`))`,
    /// so it runs only when the theme changed (the `Load` insert, or the GTW-137
    /// re-derive — repainting ALL [`Themed`](crate::theming::themed::Themed) entities = the
    /// retheme) or a new [`Themed`](crate::theming::themed::Themed) entity appeared (so a
    /// freshly-spawned widget still gets its base look), and **never** on a
    /// steady-state frame — where re-running every frame would clobber the GTW-118
    /// hover/press feedback. The `resource_exists` arm also keeps it inert and
    /// panic-free before the theme is populated (bevy-traps rule 1). The named set
    /// is the deterministic ordering anchor (bevy-traps rule 3).
    ///
    /// The GTW-137 live-retheme trigger (the GTW-564 generic
    /// [`redrive_hot_ron_resource`]`::<GdtfThemeSpec, GdtfTheme>`)
    /// runs in [`Update`] `.before(`[`UiSystems::ApplyTheme`](crate::theming::themed::UiSystems::ApplyTheme)`)`
    /// so a re-derived theme repaints the same frame (bevy-traps rule 3). It
    /// reads the [`AssetEvent`](bevy::asset::AssetEvent) **message** stream
    /// (bevy-traps rule 4) and is gated on both `GdtfTheme` and the theme
    /// asset-event message buffer existing — the buffer is registered only with
    /// `AssetPlugin` + the GTW-136 loader, so the guard keeps its `MessageReader`
    /// from failing param validation under an asset-less `MinimalPlugins` harness
    /// (bevy-traps rule 1).
    ///
    /// The GTW-118 interaction layer —
    /// [`theme_interaction`](crate::widgets::interaction::theme_interaction) (hover/press swap),
    /// [`paint_disabled_buttons`](crate::widgets::core::paint_disabled_buttons) (disabled
    /// fill), and the GTW-253
    /// [`paint_active_buttons`](crate::widgets::core::paint_active_buttons) (active / toggled-on
    /// fill, skipping disabled buttons) — runs in [`Update`] `.after(`[`UiSystems::ApplyTheme`](crate::theming::themed::UiSystems::ApplyTheme)`)`
    /// so each composes on top of the freshest base look (bevy-traps rule 3);
    /// all guard the theme internally (`Option<Res<GdtfTheme>>`), so they too
    /// are inert before the resource is populated (bevy-traps rule 1).
    ///
    /// The GTW-280 deactivation repaint
    /// ([`repaint_deactivated_buttons`](crate::widgets::interaction::repaint_deactivated_buttons))
    /// runs in the **same** `.after(UiSystems::ApplyTheme)` band: it reads
    /// [`RemovedComponents`](bevy::prelude::RemovedComponents)`<`[`ActiveButton`](crate::widgets::core::ActiveButton)`>`
    /// and repaints a button the frame it loses `ActiveButton` from its current
    /// [`Interaction`](bevy::ui::Interaction), so a just-de-selected toggle (whose
    /// `Interaction` is unchanged) does not keep a stale active fill until hovered.
    /// Its write set (`Without<ActiveButton>`) is disjoint from
    /// [`paint_active_buttons`](crate::widgets::core::paint_active_buttons)'s
    /// (`With<ActiveButton>`), so there is no write conflict between them.
    ///
    /// The GTW-147 theme-reload repaint
    /// ([`repaint_theme_change`](crate::widgets::interaction::repaint_theme_change)) runs in the
    /// **same** `.after(UiSystems::ApplyTheme)` band but is additionally gated on
    /// `resource_changed::<GdtfTheme>`: on the frame the theme changes, it re-derives
    /// every ENABLED button's [`BackgroundColor`](bevy::ui::BackgroundColor) from its
    /// CURRENT [`Interaction`](bevy::ui::Interaction) (the shared
    /// `interaction_fill` mapping), so a button held `Hovered`/`Pressed` across a
    /// hot-reload is not left on the new theme's resting base until re-hovered —
    /// [`apply_theme`](crate::theming::themed::apply_theme) ignores `Interaction`, and
    /// [`theme_interaction`](crate::widgets::interaction::theme_interaction) only fires on
    /// `Changed<Interaction>`. Its query excludes
    /// [`DisabledButton`](crate::widgets::core::DisabledButton),
    /// [`ActiveButton`](crate::widgets::core::ActiveButton),
    /// [`Segment`](crate::widgets::core::Segment), and [`Switch`](crate::widgets::core::Switch),
    /// so its write set is disjoint from the disabled/active/segment/switch paints —
    /// no two systems write the same button's fill this frame.
    ///
    /// The GTW-141 mouse hover→focus bridge
    /// ([`sync_hover_to_focus`](crate::widgets::interaction::sync_hover_to_focus)) runs in the
    /// **same** `.after(UiSystems::ApplyTheme)` band: it reads the
    /// [`Interaction`](bevy::ui::Interaction) that `bevy_ui`'s built-in
    /// `ui_focus_system` already wrote from the mouse in `PreUpdate` and moves
    /// [`InputFocus`](bevy::input_focus::InputFocus) onto the hovered button, so
    /// pointer hover and keyboard / gamepad navigation share one focus cursor.
    /// No extra picking plugin is added — `ui_focus_system` (in `DefaultPlugins`)
    /// already drives [`Interaction`](bevy::ui::Interaction), so real clicks
    /// reach GTW-122's mouse action layer with no new wiring (bevy-traps).
    ///
    /// The GTW-276 generic HUD-widget drivers run in [`Update`], independent of the
    /// theming band (they carry their own colors, not the `GdtfTheme`):
    /// [`drive_switches`](crate::widgets::core::drive_switches) flips a clicked
    /// [`Switch`](crate::widgets::core::Switch) and writes a
    /// [`ToggleFlipped`](crate::widgets::core::ToggleFlipped) message;
    /// [`select_segment_on_press`](crate::widgets::core::select_segment_on_press) sets a
    /// pressed [`SegmentedControl`](crate::widgets::core::SegmentedControl)'s active index
    /// and writes a [`SegmentSelected`](crate::widgets::core::SegmentSelected) message, and
    /// [`repaint_segments`](crate::widgets::core::repaint_segments) runs
    /// `.after(select_segment_on_press)` so it repaints ALL segments the SAME frame
    /// the active index changed (bevy-traps rule 3). Both message buffers are
    /// registered here so a downstream listener can read them.
    ///
    /// Later tickets hang further UI systems/resources here; the layer stays
    /// bevy-only (no ecosystem crate).
    fn build(&self, app: &mut App) {
        app.add_message::<ToggleFlipped>()
            .add_message::<SegmentSelected>()
            .add_systems(
                Update,
                (
                    // GTW-276 widget drivers. `repaint_segments` runs after the press
                    // handler so the de-selected segment returns to base the SAME frame
                    // the active index changes (active-driven, never hover — the
                    // GTW-280/284 lesson; bevy-traps rule 3).
                    drive_switches,
                    select_segment_on_press,
                    repaint_segments.after(select_segment_on_press),
                ),
            );
        // GTW-564: the theme's HotRonChain config (the canonical path + the
        // font-resolving `resolve_theme_spec` map hook) the generic redrive below
        // reads. Plain data — headless-safe to insert unconditionally.
        app.insert_resource(theme_hot_ron_chain());
        app.add_plugins(FocusNavPlugin).add_systems(
            Update,
            (
                // GTW-137: on a theme-asset `AssetEvent::Modified` for the active
                // handle, re-derive and overwrite `GdtfTheme` — via the GTW-564
                // GENERIC hot-RON redrive (the theme is its MAPPED chain:
                // `GdtfThemeSpec` payload -> `GdtfTheme` resource, fonts resolved
                // through the `AssetServer` map hook). Ordered BEFORE the
                // ApplyTheme set so the resulting theme-changed marks repaint the
                // same frame (bevy-traps rule 3). Gated on `GdtfTheme` existing AND
                // the asset-event message buffer existing: the latter is only
                // registered with `AssetPlugin` + `init_ron_asset`, so this keeps
                // the system's `MessageReader` from failing param validation under
                // a `MinimalPlugins` harness with no asset support (bevy-traps rule
                // 1).
                redrive_hot_ron_resource::<GdtfThemeSpec, GdtfTheme>
                    .before(UiSystems::ApplyTheme)
                    .run_if(
                        resource_exists::<GdtfTheme>
                            .and_then(resource_exists::<ThemeAssetMessages>),
                    ),
                // GTW-144: `apply_theme` is CHANGE-DRIVEN — it runs only when the
                // theme changed (the Load insert OR a GTW-137 re-derive: repaints
                // ALL Themed = the retheme) OR a new `Themed` entity appeared (so
                // freshly-spawned widgets still get their base look). On steady
                // frames it does not run, so it never clobbers the GTW-118
                // hover/press feedback that only updates on `Changed<Interaction>`.
                apply_theme.in_set(UiSystems::ApplyTheme).run_if(
                    resource_exists::<GdtfTheme>
                        .and_then(resource_changed::<GdtfTheme>.or_else(any_themed_added)),
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
                // GTW-280 — deactivation repaint: `repaint_deactivated_buttons`
                // reads `RemovedComponents<ActiveButton>` and repaints a button the
                // frame it loses `ActiveButton` from its CURRENT `Interaction`, so a
                // just-de-selected toggle (sibling became active, this button's
                // `Interaction` unchanged) does not keep its stale active fill until
                // hovered. Its write set is DISJOINT from `paint_active_buttons`: it
                // is `Without<ActiveButton>` and the active paint is
                // `With<ActiveButton>`, so the two never write the same button's
                // `BackgroundColor` — no ordering conflict between them. It runs in
                // the same `.after(UiSystems::ApplyTheme)` band so it composes on the
                // freshest base look (bevy-traps rule 3) and guards the theme as
                // `Option<Res<GdtfTheme>>` (bevy-traps rule 1).
                (
                    theme_interaction,
                    paint_disabled_buttons,
                    paint_active_buttons.after(theme_interaction),
                    repaint_deactivated_buttons,
                    sync_hover_to_focus,
                )
                    .after(UiSystems::ApplyTheme),
                // GTW-147 — theme-reload repaint: on the frame `GdtfTheme` changes,
                // `apply_theme` repaints every button to its base fill IGNORING its
                // current `Interaction`, and `theme_interaction` only fires on
                // `Changed<Interaction>` — so a button held `Hovered`/`Pressed`
                // across the reload (no later interaction change) was stuck on the
                // new theme's resting base until re-hovered.
                // `repaint_theme_change` re-derives every ENABLED button's fill from
                // its CURRENT `Interaction` (the shared `interaction_fill` mapping).
                // It runs in the same `.after(UiSystems::ApplyTheme)` band so it
                // composes on the freshest base (bevy-traps rule 3), gated on
                // `resource_changed::<GdtfTheme>` so it does nothing on steady frames
                // (where it would clobber per-frame hover feedback) and on
                // `resource_exists::<GdtfTheme>` so its `Res<GdtfTheme>` never fails
                // param validation before the theme is populated (bevy-traps rule 1).
                // Its write set is DISJOINT from the special paints above
                // (`Without<DisabledButton/ActiveButton/Segment/Switch>`), so no two
                // systems write the same button's `BackgroundColor` this frame.
                repaint_theme_change
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<GdtfTheme>.and_then(resource_changed::<GdtfTheme>)),
            ),
        );
    }
}
