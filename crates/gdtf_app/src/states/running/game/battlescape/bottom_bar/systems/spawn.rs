//! Spawns + despawns the battlescape BOTTOM BAR (GTW-275 layout overhaul, item 6; GTW-298
//! screenshot review 2026-06-18 — the bar is now a THEMED PANEL).
//!
//! [`spawn_bottom_bar`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds the ONE
//! full-width strip at the bottom of the screen — the only UI that reduces the world map
//! (item 1 / item 4: the status / hover panels are corner OVERLAYS that do not inset the map;
//! the map ends at this bar's top edge, measured by `set_world_viewport`). The weapon + stance
//! cluster sits INSIDE this strip (a z-stacked overlay parented elsewhere); this module builds
//! the bar itself.
//!
//! GTW-298 (screenshot review 2026-06-18): the bar is now a proper THEMED PANEL
//! ([`spawn_panel`](gdtf_ui::spawn_panel) / `Themed(Panel)`) rather than a bare opaque Node, so
//! the whole bottom area reads as ONE bordered panel containing the weapon + stance cluster, like
//! the other HUD panels. `apply_theme` repaints its border / radius / panel background every run;
//! the absolute full-width-bottom anchor + responsive height + z below are layout fields that
//! survive the theme pass.
//!
//! [`despawn_bottom_bar`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the bar by its [`BottomBarRoot`] marker — battle-scoped, mirroring the sibling
//! status panel / action bar / weapon panel lifecycle.

use bevy::{
    color::Alpha,
    prelude::*,
    ui::{BackgroundColor, GlobalZIndex, Node, PositionType, UiRect, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::bottom_bar::components::{
    BOTTOM_BAR_H_VH, BottomBarRoot, bottom_bar_padding,
};

/// The bottom bar's stacking order ([`GlobalZIndex`] — the higher, the nearer the viewer).
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`] (the framework carve-out, not
/// a domain value). The corner status / hover panels default to `0`, so the bar sits ABOVE the
/// world map yet the weapon + stance cluster (at a higher z) draws on top of the panel fill — a
/// deterministic z-stack independent of spawn order (no `GlobalZIndex` precedent existed in the
/// battlescape, so this establishes the bottom-strip stacking).
const BOTTOM_BAR_Z: i32 = 10;

/// Builds the bottom-bar THEMED PANEL on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — in the running app the theme is present by the time a battle starts;
/// the status-panel / weapon-panel precedent). With the theme present it spawns a
/// [`spawn_panel`](gdtf_ui::spawn_panel) THEMED PANEL (`Themed(Panel)` — bordered, panel fill /
/// radius re-painted by `apply_theme`) and overwrites its [`Node`] with the bar layout: a
/// [`PositionType::Absolute`] strip anchored to the window bottom, FULL window WIDTH
/// ([`Val::Vw`](bevy::ui::Val)`(100.0)`) at the shared responsive height [`BOTTOM_BAR_H_VH`]. So
/// the whole bottom area reads as ONE bordered panel containing the weapon + stance cluster, like
/// the other HUD panels (GTW-298 screenshot review 2026-06-18). The layout fields (position /
/// size) survive the theme pass — `apply_theme` overrides only the theme-owned border / radius /
/// padding for the Panel role; the four-sided content `padding` set here ([`bottom_bar_padding`])
/// is re-applied after each theme pass by `repad_bottom_bar` so the content inset survives a
/// repaint. Its measured height is what insets the map (item 1).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] + the theme read.
pub(in crate::states::running::game::battlescape) fn spawn_bottom_bar(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed bar (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // The bottom area is ONE themed panel (GTW-298): `spawn_panel` paints the panel look
    // (border / fill / radius), then we overwrite the Node with the absolute full-width-bottom
    // bar layout. `apply_theme` re-applies the theme-owned border / radius / padding for the
    // Panel role every run, preserving these layout fields (the weapon-panel-root precedent).
    let bar = spawn_panel(&mut commands, &theme);
    // GTW-298: the bottom strip is the ONE solid panel the weapon + stance cluster sit in,
    // hovering over the DOTTED map — so its fill must be OPAQUE (the theme's panel fill is
    // `alpha 0.55`, which bled the map dots through — R3). Force the fill alpha to 1.0 at spawn
    // (so it is opaque on the very first frame); `opacify_bottom_bar` keeps it opaque after each
    // theme repaint. The theme still owns the border / radius (re-applied every run).
    let mut opaque_fill = *theme.panel.color;
    opaque_fill.set_alpha(1.0);
    commands.entity(bar).insert((
        BottomBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::ZERO,
            left: Val::ZERO,
            width: Val::Vw(100.0),
            height: Val::Vh(BOTTOM_BAR_H_VH),
            border: UiRect::all(Val::Vw(*theme.panel.border_width)),
            // Four-sided breathing room so the weapon cluster + stance column inset off EVERY
            // edge (especially the bottom row: Prone / firemode buttons / Aim are not flush
            // against the window bottom), matching the mockup. Relative units (Vw/Vh) only.
            // Set here so it is right on the FIRST frame; `repad_bottom_bar` re-applies it after
            // each theme pass (`apply_theme`'s `box_node` overwrites `padding`).
            padding: bottom_bar_padding(),
            ..default()
        },
        BackgroundColor(opaque_fill),
        // Stacks ABOVE the world map; the weapon + stance cluster (a higher z) draws on top of
        // the panel fill — deterministic, not spawn-order dependent.
        GlobalZIndex(BOTTOM_BAR_Z),
    ));
}

/// Despawns the bottom bar on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`BottomBarRoot`] entity so the bar is gone the moment the battle
/// leaves `BattleRunning` — battle-scoped lifecycle. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<BottomBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_bottom_bar(
    mut commands: Commands,
    bars: Query<Entity, With<BottomBarRoot>>,
) {
    for bar in &bars {
        commands.entity(bar).despawn();
    }
}
