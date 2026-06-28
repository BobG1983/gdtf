//! Spawns + despawns the Prev/Next selection-cycle cluster (GTW-458) — the far-RIGHT
//! vertical button pair on the bottom bar.
//!
//! [`spawn_select_cycle`] runs `OnEnter(BattleScapeState::BattleRunning)` ordered
//! `.after(spawn_bottom_bar)` (the bar root must exist so the cluster parents INSIDE it — the
//! Stance-Panel precedent). It builds a [`SelectCycleRoot`] vertical
//! [`Column`](bevy::ui::FlexDirection::Column) anchored to the bar's RIGHT edge, sized to a
//! fixed relative width `<= 10%` of the WINDOW ([`CLUSTER_W_VW`]) and the SAME content-box
//! height as the other on-bar panels ([`cluster_height_vh`]), holding two themed
//! [`spawn_button`](gdtf_ui::spawn_button)s — **Next** (top, up glyph) over **Prev** (bottom,
//! down glyph) — each `~50%` of the cluster height. It is parented as a CHILD of the
//! [`BottomBarRoot`] (so the bar's own despawn tears it down) and sits INSIDE the bar's
//! existing padding, so it does NOT change the bar's measured height (C1). Relative units only
//! (`Vw`/`Vh`/`Percent`), no fixed px but the font.
//!
//! [`despawn_select_cycle`] runs `OnExit(BattleScapeState::BattleRunning)` and despawns the
//! cluster by its [`SelectCycleRoot`] marker — covering the defensive fallback where the bar
//! root was absent and the cluster was parented under nothing-but-itself.

use bevy::{
    prelude::*,
    ui::{FlexDirection, GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{ButtonLabel, spawn_button, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    bottom_bar::{BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_X_VW, BOTTOM_BAR_PAD_Y_VH, BottomBarRoot},
    select_cycle::components::{SelectCycleRoot, SelectNextButton, SelectPrevButton},
};

/// The WIDTH of the selection-cycle cluster, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val)) — the contract's `<= 10%` cap (C1).
///
/// A `const`, layout plumbing fed straight to a [`Node`] (the `CELL_PX`-class carve-out, not a
/// domain value). A narrow far-right cluster; `Vw` so it scales with the window. Held at the
/// 10% ceiling exactly so the two stacked buttons read at a usable width without exceeding the
/// cap.
const CLUSTER_W_VW: f32 = 10.0;

/// The selection-cycle cluster's stacking order ([`GlobalZIndex`]) — one ABOVE the bottom
/// bar's (`bottom_bar` = 10), so the buttons draw on TOP of the bar's opaque fill, matching the
/// weapon / stance cluster z (11).
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`] (the framework carve-out,
/// not a domain value).
const CLUSTER_Z: i32 = 11;

/// The Next / Prev button height as a `Percent` of the cluster — each takes HALF the column so
/// the two are evenly stacked (C1: each `~50%` of the cluster height). A `const` layout
/// plumbing value, NOT a fixed px.
const BUTTON_H_PCT: f32 = 50.0;

/// The cluster HEIGHT as a fraction of the WINDOW HEIGHT ([`Val::Vh`](bevy::ui::Val)) — the
/// bar's CONTENT-box height (the bar height MINUS its top + bottom padding), the SAME height
/// the weapon / stance clusters use so the cluster stays inside the bar's border + padding and
/// never changes the bar's measured height (C1).
///
/// A `const fn` so the value is computed from the shared bottom-bar consts (one source of
/// truth); layout plumbing, not a domain value (the `CELL_PX`-class carve-out).
const fn cluster_height_vh() -> f32 {
    BOTTOM_BAR_H_VH - 2.0 * BOTTOM_BAR_PAD_Y_VH
}

/// Builds one cycle button (Next / Prev) — a themed [`spawn_button`](gdtf_ui::spawn_button)
/// carrying `marker`, captioned `label`, sized to FULL cluster width and HALF the cluster
/// height. Returns the button [`Entity`].
///
/// The caption uses an arrow glyph so the pair reads as up = Next / down = Prev at a glance;
/// the button's themed look is painted by `apply_theme` (the layout `Node` overwrite survives
/// the theme pass — the weapon-panel item-button precedent).
fn spawn_cycle_button(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: &str,
    marker: impl Bundle,
) -> Entity {
    let button = spawn_button(commands, theme, ButtonLabel::new(label), marker);
    commands.entity(button).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(BUTTON_H_PCT),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    button
}

/// Builds the Prev/Next cluster on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1 — the bottom-bar / weapon-panel precedent). With the theme present it
/// spawns the [`SelectCycleRoot`] vertical column anchored to the bar's RIGHT edge (`right: 0`,
/// width [`CLUSTER_W_VW`] `<= 10%`, height [`cluster_height_vh`]) holding **Next** over
/// **Prev**, each half the column height. It is parented as a CHILD of the [`BottomBarRoot`]
/// (so it sits INSIDE the bar's padding and the bar's despawn tears it down); if the bar root
/// is somehow absent (defensive — this system is ordered `.after(spawn_bottom_bar)`) the
/// cluster still spawns as a free root so the buttons exist.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the theme read, and a read-only
/// `Query<Entity, With<BottomBarRoot>>`.
pub(in crate::states::running::game::battlescape) fn spawn_select_cycle(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    bottom_bar: Query<Entity, With<BottomBarRoot>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed cluster (the bottom-bar
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    let next = spawn_cycle_button(&mut commands, &theme, "^ Next", SelectNextButton);
    let prev = spawn_cycle_button(&mut commands, &theme, "v Prev", SelectPrevButton);

    // The cluster root: a vertical column anchored to the bar's RIGHT edge. It is an ABSOLUTE
    // child of the bar (whose `right` is relative to the bar's right edge = the window right,
    // the bar being full-width). Its fixed `<= 10%` width keeps it from changing the bar's
    // layout, and its content-box height keeps it inside the bar's border + padding so the bar
    // height is unchanged (C1). `align_items: FlexEnd` right-aligns the buttons within the
    // (full-width) column for the far-right reading.
    let root = commands
        .spawn((
            SelectCycleRoot,
            Node {
                position_type: PositionType::Absolute,
                // Anchor a bottom-padding ABOVE the window bottom (the weapon / stance cluster
                // precedent), so the cluster insets off the bar's bottom edge rather than
                // sitting flush against the window edge.
                bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
                // Inset off the bar's RIGHT edge by the bar's own horizontal padding (a
                // `Vw`), so the cluster sits inside the bar's right gutter rather than flush
                // against the window edge — matching the weapon cluster's left inset.
                right: Val::Vw(BOTTOM_BAR_PAD_X_VW),
                width: Val::Vw(CLUSTER_W_VW),
                height: Val::Vh(cluster_height_vh()),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            // Draw ON TOP of the bottom bar's opaque fill (one z above the bar — the weapon /
            // stance cluster precedent).
            GlobalZIndex(CLUSTER_Z),
        ))
        .id();
    commands.entity(root).add_children(&[next, prev]);

    // Parent the cluster INSIDE the bottom bar (the Stance-Panel precedent). This system is
    // ordered `.after(spawn_bottom_bar)`, so the bar root exists; if it is somehow absent
    // (defensive) the cluster stays a free root so the buttons still spawn.
    if let Some(bar) = bottom_bar.iter().next() {
        commands.entity(bar).add_children(&[root]);
    }
}

/// Despawns the selection-cycle cluster on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`SelectCycleRoot`] (and so its buttons) so the cluster is gone the
/// moment the battle leaves `BattleRunning` — battle-scoped lifecycle. When the cluster is a
/// child of the bottom bar, `despawn_bottom_bar` also tears it down; this still covers the
/// defensive fallback where the bar was absent and the cluster spawned as a free root.
/// Param-only (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<SelectCycleRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_select_cycle(
    mut commands: Commands,
    clusters: Query<Entity, With<SelectCycleRoot>>,
) {
    for cluster in &clusters {
        commands.entity(cluster).despawn();
    }
}
