//! Spawns + despawns the battlescape status HUD panel (GTW-275).
//!
//! [`spawn_status_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] panel on the GTW-120 UI camera: a [`spawn_panel`] box
//! ([`StatusPanelRoot`]) anchored TOP-LEFT, holding the one shared
//! [`stat_block`](super::super::super::stat_block) subtree (portrait / name / faction /
//! stance / TU+HP `ProgressBar`s / Wounds `Pips` / wound-name list). The widgets start at
//! empty / zero values and [`update_status_panel`](super::update::update_status_panel)
//! repaints them from the selection every battle frame by mutating the existing widgets.
//!
//! Layout (GTW-275): the root is an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`) OVERLAY anchored
//! to the window's TOP-LEFT corner, hovering OVER the full-window map. It is `PositionType::
//! Absolute` so it is removed from layout flow and contributes NOTHING to the world-map
//! viewport inset — only the bottom bar reduces the map (the layout-model correction). Its
//! width is a FIXED fraction of the viewport so the map never pops/shifts left-right when the
//! panel's contents change (a different selected ganger), and its height tracks the content
//! but is CAPPED in `Val::Vh` with overflow clipped so it can never overrun the window. This
//! mirrors the sibling top-right [`inspect_panel`](super::super::super::inspect_panel).
//!
//! [`despawn_status_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole panel by its [`StatusPanelRoot`] marker, so the panel is
//! battle-scoped: present only during the live tactical layer, gone the moment the battle
//! leaves `BattleRunning`. The battlescape neighborhood uses explicit `OnExit` cleanup,
//! mirroring the sibling action-bar.

use bevy::{
    prelude::*,
    ui::{Node, OverflowAxis, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    stat_block::spawn_stat_block,
    status_panel::components::{StatusPanelRoot, StatusStatBlock},
};

/// The status panel's **fixed** width as a fraction of the viewport WIDTH
/// ([`Val::Vw`](bevy::ui::Val::Vw)). A relative unit (`ui-responsive-not-px`), and *fixed* so
/// the map area never pops/shifts left-right when the panel's contents change — the selected
/// ganger's stat block always occupies this one width (item 7). The panel is an absolute
/// overlay (item 2), so this width contributes NOTHING to the world-map viewport inset
/// (only the bottom bar reduces the map — item 4). Matches the sibling inspect panel's width.
const PANEL_WIDTH_VW: f32 = 18.0;

/// The status panel's **maximum** height as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)). The panel height tracks its stat-block content
/// (`height: auto`), but is CAPPED here so it can never overrun the window; the cap is
/// relative (`ui-responsive-not-px`). Excess content is clipped (overflow hidden) rather than
/// overflowing onto the map. Matches the sibling inspect panel's cap.
const PANEL_MAX_HEIGHT_VH: f32 = 45.0;

/// Builds the themed status panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time a battle
/// starts. With the theme present it spawns the panel root via [`spawn_panel`] (a
/// `Themed(Panel)` box), tagged [`StatusPanelRoot`], laid out as an ABSOLUTE, fixed-%
/// (`Val::Vw`/`Val::Vh`) OVERLAY anchored TOP-LEFT (GTW-275), and parents the shared stat
/// block ([`spawn_stat_block`]) under it. The portrait atlas is read off the presenter's
/// [`TopDownAtlases`] as `Option<Res<…>>` so the panel still builds before the atlas loads.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme / atlas reads.
pub(in crate::states::running::game::battlescape) fn spawn_status_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the action-bar
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: an ABSOLUTE, fixed-% overlay anchored TOP-LEFT, hovering OVER the map (item 2).
    // It is a UI node on the over-map UI camera, positioned `Absolute` so it is removed from
    // any layout flow and contributes NOTHING to the world-map viewport inset — only the
    // bottom bar reduces the map (item 4). Its width is a FIXED fraction of the viewport
    // (`Val::Vw`, `ui-responsive-not-px`) so the map never pops/shifts when the panel's
    // contents change (item 7); its height tracks the stat block but is CAPPED in `Val::Vh`
    // and clips overflow so it can never overrun the map. Replacing the builder's `Node` is
    // fine: `apply_theme` re-derives the panel's border / radius / padding from this node
    // every run (it clones the node and overrides only those), so the framed look is
    // preserved while the size + position here stay authoritative. Mirrors the sibling
    // top-right `inspect_panel`.
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        StatusPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Vh(0.0),
            left: Val::Vw(0.0),
            width: Val::Vw(PANEL_WIDTH_VW),
            height: Val::Auto,
            max_height: Val::Vh(PANEL_MAX_HEIGHT_VH),
            flex_direction: FlexDirection::Column,
            overflow: Overflow {
                x: OverflowAxis::Hidden,
                y: OverflowAxis::Hidden,
            },
            ..default()
        },
    ));

    // The one shared stat block, marked as THIS panel's and parented under the panel root.
    let block = spawn_stat_block(&mut commands, &theme, atlases.as_deref());
    commands.entity(block).insert(StatusStatBlock);
    commands.entity(root).add_children(&[block]);
}

/// Despawns the status panel on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`StatusPanelRoot`] entity (and so its stat-block children) so
/// the panel is gone the moment the battle leaves `BattleRunning` — battle-scoped
/// lifecycle. Param-only (`bevy-traps.md` #7): [`Commands`] + a
/// `Query<Entity, With<StatusPanelRoot>>`.
pub(in crate::states::running::game::battlescape) fn despawn_status_panel(
    mut commands: Commands,
    panels: Query<Entity, With<StatusPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
