//! The [`UiPlugin`] registration seam and its theme-asset message-buffer gate.

use bevy::{
    prelude::*,
    ui_widgets::{ScrollAreaPlugin, ScrollbarPlugin},
};
use gdtf_assets::RonAsset;

use crate::{
    focus_nav::{FocusNavPlugin, FocusNavSystems},
    theming::{
        retheme::redrive_theme_on_asset_event,
        theme::{GdtfTheme, GdtfThemeSpec},
        themed::{UiSystems, any_themed_added, apply_theme},
    },
    widgets::{
        core::{
            DropdownDismissRequest, DropdownSelectionChanged, OptionId, SegmentSelected,
            ToggleFlipped, activate_focused_option, close_dropdowns_on_dismiss_request,
            dismiss_dropdowns_on_escape, dismiss_on_backdrop_press, drive_switches, open_dropdown,
            paint_active_buttons, paint_disabled_buttons, position_dropdown_popups,
            repaint_segments, select_option_on_press, select_segment_on_press,
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

/// The GDTF UI plugin — the single registration seam for the hand-rolled UI.
///
/// Added once by `gdtf_app::GdtfApp` (and mirrored by the headless test-support
/// path), this is where every later UI ticket hangs its systems, resources, and
/// observers. Keeping the seam in place from the start means downstream wiring
/// changes touch only [`build`](UiPlugin::build), never the app's plugin list.
///
/// It currently installs the focus-navigation layer
/// ([`FocusNavPlugin`](crate::focus_nav::FocusNavPlugin)), the central theming pass
/// ([`apply_theme`](crate::theming::themed::apply_theme)), the GTW-137 live-retheme trigger
/// ([`redrive_theme_on_asset_event`](crate::theming::retheme::redrive_theme_on_asset_event)),
/// the GTW-118 widget interaction layer
/// ([`theme_interaction`](crate::widgets::interaction::theme_interaction) +
/// [`paint_disabled_buttons`](crate::widgets::core::paint_disabled_buttons)), and the
/// GTW-141 mouse hover→focus bridge
/// ([`sync_hover_to_focus`](crate::widgets::interaction::sync_hover_to_focus)).
///
/// For the GTW-410 dropdown it installs only the TYPE-AGNOSTIC pieces (the
/// [`DropdownDismissRequest`](crate::DropdownDismissRequest) message, the `Escape` emitter
/// [`dismiss_dropdowns_on_escape`](crate::dismiss_dropdowns_on_escape), and the popup
/// positioner [`position_dropdown_popups`](crate::position_dropdown_popups)); a dropdown is
/// generic over its option identity, so each caller registers its concrete option-id type's
/// drivers + message via [`register_dropdown::<T>`](register_dropdown).
///
/// For the GTW-412 [`ScrollList`](crate::ScrollList) it ensures the built-in scroll widgets'
/// plugins ([`ScrollAreaPlugin`] + [`ScrollbarPlugin`]) are present — added only if a
/// `DefaultPlugins` app (whose `UiWidgetsPlugins` already supply them) has not registered
/// them, since a unique plugin re-add panics.
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
    /// The GTW-137 live-retheme trigger
    /// ([`redrive_theme_on_asset_event`](crate::theming::retheme::redrive_theme_on_asset_event))
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
        // GTW-412 — the built-in scroll widgets the `ScrollList` container relies on:
        // `ScrollAreaPlugin` registers the `Pointer<Scroll>` observer that clamps
        // `ScrollPosition` to the overflow, and `ScrollbarPlugin` registers
        // `update_scrollbar_thumb` (in `PostUpdate`, after `ui_layout_system`) that sizes +
        // positions the thumb from the content/viewport ratio. These ride
        // `DefaultPlugins`' `UiWidgetsPlugins` (the workspace `ui` feature pulls
        // `bevy_ui_widgets`), so under the real app + the `DefaultPlugins` test harness they
        // are ALREADY present — re-adding a unique plugin panics. The `is_plugin_added`
        // guard keeps `UiPlugin` correct under BOTH a `DefaultPlugins` app (present → skip)
        // and a bare app that wired `UiPlugin` without the widget group (absent → add), so
        // the `ScrollList` scroll mechanism is never silently missing. (This deviates from
        // the ticket's literal "always add" wording, which would panic here — the intent,
        // "wire the engine scroll widgets, don't hand-roll", is met.)
        if !app.is_plugin_added::<ScrollAreaPlugin>() {
            app.add_plugins(ScrollAreaPlugin);
        }
        if !app.is_plugin_added::<ScrollbarPlugin>() {
            app.add_plugins(ScrollbarPlugin);
        }
        app.add_message::<ToggleFlipped>()
            .add_message::<SegmentSelected>()
            // GTW-410 — the dropdown's ONE type-agnostic message (the Esc-close request);
            // the per-option-id `DropdownSelectionChanged<T>` message + generic drivers are
            // registered per concrete `T` via `register_dropdown::<T>` (an unregistered
            // generic system never runs — gate 4b).
            .add_message::<DropdownDismissRequest>()
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
                    // GTW-410 — the type-agnostic dropdown systems: the `Escape`-press emitter
                    // and the per-frame popup positioner (it reads the trigger's
                    // `UiGlobalTransform` + `ComputedNode` to place the floating list flush
                    // below the closed control — bevy-traps #8).
                    dismiss_dropdowns_on_escape,
                    position_dropdown_popups,
                ),
            );
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

/// Registers the [`Dropdown<T>`](crate::Dropdown) drivers + message for ONE concrete option
/// identity type `T` (GTW-410).
///
/// A dropdown is generic over its option identity, and a generic system cannot be added for
/// an arbitrary unknown `T` — so [`UiPlugin`] registers only the dropdown's type-agnostic
/// pieces (the [`DropdownDismissRequest`] message + the `Escape` emitter + the popup
/// positioner), and each caller (or test) calls `register_dropdown::<MyId>(app)` once per
/// option-id type it spawns. Without this the per-type systems never run (an unregistered
/// system is a dead feature — gate 4b).
///
/// It registers the [`DropdownSelectionChanged<T>`](crate::DropdownSelectionChanged) message
/// and adds, in [`Update`]:
///
/// - [`open_dropdown::<T>`](crate::open_dropdown) — click toggles the floating list open/closed.
/// - [`select_option_on_press::<T>`](crate::select_option_on_press) ordered
///   `.before(open_dropdown::<T>)` so an option press that closes the list is NOT re-opened by
///   the trigger handler the same frame (the recipe's ordering; bevy-traps rule 3).
/// - [`activate_focused_option::<T>`](crate::activate_focused_option) ordered
///   `.after(`[`FocusNavSystems::Bridge`](crate::focus_nav::FocusNavSystems::Bridge)`)` so the
///   `Enter` → [`FocusActivated`](crate::focus_nav::FocusActivated) message is populated before
///   it is drained (bevy-traps rule 3) — keyboard select.
/// - [`dismiss_on_backdrop_press::<T>`](crate::dismiss_on_backdrop_press) — outside-click dismiss.
/// - [`close_dropdowns_on_dismiss_request::<T>`](crate::close_dropdowns_on_dismiss_request) — the
///   `Escape` consumer (a no-op when nothing is open).
pub fn register_dropdown<T: OptionId>(app: &mut App) {
    app.add_message::<DropdownSelectionChanged<T>>()
        .add_systems(
            Update,
            (
                open_dropdown::<T>,
                select_option_on_press::<T>.before(open_dropdown::<T>),
                activate_focused_option::<T>.after(FocusNavSystems::Bridge),
                dismiss_on_backdrop_press::<T>,
                close_dropdowns_on_dismiss_request::<T>,
            ),
        );
}
