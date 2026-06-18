//! Spawns + despawns the battlescape status HUD panel (GTW-278).
//!
//! [`spawn_status_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a
//! themed [`gdtf_ui`] panel on the GTW-120 UI camera: a [`spawn_panel`] box
//! ([`StatusPanelRoot`]) anchored TOP-LEFT, holding the one shared
//! [`stat_block`](super::super::super::stat_block) subtree (portrait / name / faction /
//! stance / TU+HP `ProgressBar`s / Wounds `Pips` / wound-name list). The widgets start at
//! empty / zero values and [`update_status_panel`](super::update::update_status_panel)
//! repaints them from the selection every battle frame by mutating the existing widgets.
//!
//! [`despawn_status_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole panel by its [`StatusPanelRoot`] marker, so the panel is
//! battle-scoped: present only during the live tactical layer, gone the moment the battle
//! leaves `BattleRunning`. The battlescape neighborhood uses explicit `OnExit` cleanup,
//! mirroring the sibling action-bar.

use bevy::{
    prelude::*,
    ui::{Node, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{spawn_panel, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::{
    stat_block::spawn_stat_block,
    status_panel::components::{StatusPanelRoot, StatusStatBlock},
};

/// Builds the themed status panel on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time a battle
/// starts. With the theme present it spawns the panel root via [`spawn_panel`] (a
/// `Themed(Panel)` box), tagged [`StatusPanelRoot`], laid out as a TOP-LEFT absolute
/// column, and parents the shared stat block ([`spawn_stat_block`]) under it. The portrait
/// atlas is read off the presenter's [`TopDownAtlases`] as `Option<Res<…>>` so the panel
/// still builds before the atlas loads.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawns + the theme / atlas reads.
pub(in crate::scenes::running::game::battlescape) fn spawn_status_panel(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    atlases: Option<Res<TopDownAtlases>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed panel (the action-bar
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    // ROOT: the top-left anchored panel box. `spawn_panel` paints the panel look; the
    // layout fields survive `apply_theme` (it overrides only the theme-owned border /
    // radius / padding for the Panel role — the action-bar panel precedent).
    let root = spawn_panel(&mut commands, &theme);
    commands.entity(root).insert((
        StatusPanelRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
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
pub(in crate::scenes::running::game::battlescape) fn despawn_status_panel(
    mut commands: Commands,
    panels: Query<Entity, With<StatusPanelRoot>>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
