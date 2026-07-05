//! The weapon cluster's `OnEnter`/`OnExit` lifecycle pair: the bottom-left Overall
//! Weapon Panel root + the separate Stance Panel placement, and the battle-scoped
//! teardown. Split out of the monolithic `spawn.rs` (GTW-583); the authoritative
//! layout doc lives on the parent `spawn` module.

use bevy::{
    prelude::*,
    ui::{GlobalZIndex, Node, Val},
};
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use super::{
    columns::{spawn_left_column, spawn_right_column},
    geometry::{GAP_VH, GAP_VW, PANEL_H_VH, PANEL_W_VW, PANEL_Z, STANCE_LEFT_VW, STANCE_W_VW},
};
use crate::states::running::game::battlescape::{
    action_bar::spawn_stance_panel,
    bottom_bar::{BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    weapon_panel::components::WeaponPanelRoot,
};

/// Builds the themed weapon cluster on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — the status-panel precedent). With the theme present it spawns the
/// [`WeaponPanelRoot`] Overall Weapon Panel anchored BOTTOM-LEFT (sized responsively
/// [`PANEL_W_VW`] × [`PANEL_H_VH`]) as a 2×2 grid — [`spawn_left_column`] (Combined + Firemode)
/// beside [`spawn_right_column`] (Item + Aim) — and a SEPARATE Stance Panel
/// ([`spawn_stance_panel`], its OWN bordered `Themed(Panel)`) to its RIGHT (fixed-% width, height =
/// [`PANEL_H_VH`] — the SAME height as the Overall Weapon Panel, NOT the full bottom-bar height — D-B),
/// parented as a CHILD of the [`BottomBarRoot`] container so it is laid out INSIDE the
/// bottom panel rather than floating over it — and so all three stance toggles stay contained within
/// the bar's border/padding. The relocated controls (firemode / aim / stance) keep their action-bar markers, so the
/// existing press → intent + active-mark systems drive them. It is ordered
/// `.after(spawn_bottom_bar)` (the bottom-bar root must exist before the stance panel is parented
/// under it); if the bar root is somehow absent it falls back to parenting the stance under the
/// weapon root (so the toggles still spawn). Param-only (`bevy-traps.md` #7): [`Commands`], the
/// theme read, and a read-only `Query<Entity, With<BottomBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn spawn_weapon_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: the bottom-left anchored Overall Weapon Panel box (a 2×2 grid), sized responsively
    // so the grid cells resolve their `Percent` shares. `spawn_panel` paints the panel look; the
    // layout fields survive `apply_theme` (it overrides only the theme-owned border / radius /
    // padding for the Panel role — the status-panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        WeaponPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            // Anchor the cluster a bottom-padding's worth ABOVE the WINDOW bottom (A FAIL,
            // 2026-06-18 screenshot review): this root is an absolute overlay positioned
            // relative to the WINDOW (it is NOT an in-flow child of the bar, so the bar's
            // `Node::padding` does NOT inset it) — at `bottom: 0` the Firemode toggles / Aim row
            // sat FLUSH against the window's bottom edge. Lift it by the bar's own bottom-padding
            // ([`BOTTOM_BAR_PAD_Y_VH`], a window-relative `Vh`) so the bottom row insets off the
            // window's bottom edge, matching the bar's content inset (the mockup's clear gap below
            // the bottom-panel content).
            bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
            left: Val::ZERO,
            width: Val::Vw(PANEL_W_VW),
            // Height = the bar's CONTENT-box height ([`PANEL_H_VH`] = bar height MINUS its top +
            // bottom padding); anchored a bottom-padding above the window bottom, its TOP lands a
            // top-padding below the bar's top edge — so it insets off BOTH vertical edges and never
            // bleeds out the top (GTW-275 layout overhaul item 6).
            height: Val::Vh(PANEL_H_VH),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Vw(GAP_VW),
            ..default()
        },
        // Draw ON TOP of the bottom bar's opaque fill (one z above the bar — item 6).
        GlobalZIndex(PANEL_Z),
    ));

    let left = spawn_left_column(&mut commands, &theme);
    let right = spawn_right_column(&mut commands, &theme);

    commands.entity(root).add_children(&[left, right]);

    // The SEPARATE Stance Panel (GTW-298; D-B per the 2026-06-18 screenshot review): its OWN
    // bordered `Themed(Panel)` box, sitting to the RIGHT of the Overall Weapon Panel at a fixed-%
    // width — so it does not change the Overall panel's own grid width (item 7) and does not overlap
    // it (item 8). It is parented as a CHILD of the [`BottomBarRoot`] container, so it is laid out
    // INSIDE the bottom panel (contained by the bar's bounds + stacking context) rather than
    // floating ON TOP of the bottom panel's right edge as a sibling overlay of the weapon root. It
    // is an absolute child of the bar whose `left` is relative to the bar's left edge (= the window
    // left, since the bar is full-width at `left: 0`), so the same `STANCE_LEFT_VW` geometry still
    // places it just past the weapon cluster. D-B: its height is `Vh(PANEL_H_VH)` — the SAME height
    // as the Overall Weapon Panel (NOT the full bottom-bar height) — anchored a bottom-padding ABOVE
    // the window bottom (`bottom: Vh(BOTTOM_BAR_PAD_Y_VH)`, matching the Overall Weapon Panel root —
    // B FAIL, 2026-06-18 screenshot review: at `bottom: 0` the absolute inset did NOT pick up the
    // bar's bottom padding, so the framed stance box ran flush to the window's bottom edge and read
    // as loose buttons on the bar fill). Since `PANEL_H_VH` is the bar CONTENT-box height
    // (`BOTTOM_BAR_H_VH - 2 * BOTTOM_BAR_PAD_Y_VH`), the column (and so the third 'Prone' toggle)
    // stays ENTIRELY inside the bar's border + padding instead of overrunning the bottom edge — and
    // the box's own bordered frame now sits clear of the window edge, reading as a distinct
    // sub-panel. It wraps the three relocated stance toggles; the `sync_stance_buttons_active` +
    // `action_bar_button_intents` systems drive them parent-agnostically. The bottom bar's own
    // `despawn_bottom_bar` tears it down with the bar (and if the fallback parents it under the
    // weapon root, `despawn_weapon_panel` does).
    let stance = spawn_stance_panel(&mut commands, &theme);
    commands.entity(stance).insert(Node {
        position_type: PositionType::Absolute,
        // D-B (2026-06-18 screenshot review): the Stance Panel is its OWN bordered sub-panel sized
        // to the SAME HEIGHT as the Overall Weapon Panel (NOT the full bottom-bar height) and a
        // sensible fixed relative width ([`STANCE_W_VW`]). It is anchored a bottom-padding ABOVE the
        // window bottom (`bottom: Vh(BOTTOM_BAR_PAD_Y_VH)` — the EXACT same `bottom` the Overall
        // Weapon Panel root uses) with `height: Vh(PANEL_H_VH)` — the same height field — so the two
        // bordered panels are flush-bottomed with each other AND both inset off the window's bottom
        // edge (B FAIL fix: `bottom: 0` left the framed box flush against the window edge). Since
        // `PANEL_H_VH` is the bar CONTENT-box height (`BOTTOM_BAR_H_VH - 2 * BOTTOM_BAR_PAD_Y_VH`),
        // the column (and so the third 'Prone' toggle) stays inside the bar's border + padding.
        bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
        left: Val::Vw(STANCE_LEFT_VW),
        width: Val::Vw(STANCE_W_VW),
        // Height = the Overall Weapon Panel's height EXACTLY (`Vh(PANEL_H_VH)`) — the bar
        // content-box height, so the column cannot overrun the bar's bottom border/padding.
        height: Val::Vh(PANEL_H_VH),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(GAP_VH),
        ..default()
    });
    // Parent the stance panel INSIDE the bottom bar (D4). `spawn_weapon_panel` is ordered
    // `.after(spawn_bottom_bar)`, so the bar root exists; if it is somehow absent (defensive)
    // fall back to the weapon root so the stance toggles still spawn.
    let stance_parent = bottom_bar.iter().next().unwrap_or(root);
    commands.entity(stance_parent).add_children(&[stance]);
}

/// Despawns the weapon cluster on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`WeaponPanelRoot`] Overall Weapon Panel (and so its grid children)
/// so the cluster is gone the moment the battle leaves `BattleRunning` — battle-scoped lifecycle.
/// The Stance Panel is now a child of the [`BottomBarRoot`] (D4 reparent), so `despawn_bottom_bar`
/// tears it down with the bar; this still covers the defensive fallback where the bar was absent
/// and the stance was parented under the weapon root. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<WeaponPanelRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_weapon_panel(
    mut commands: Commands,
    panels: Query<Entity, With<WeaponPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
