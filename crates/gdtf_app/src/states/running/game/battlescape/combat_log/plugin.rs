//! The combat-log scene-plugin (GTW-328, slice 3, bottom-left, ABOVE the weapon panel;
//! GTW-572 rebuilds the drain as the forwarder → appender message pipeline; GTW-620 moves the
//! forwarder half down into the presenter).
//!
//! Registers the battle-scoped combat-text LOG in the battlescape neighborhood. The log
//! shows the most-recent combat events as lines that scroll up and fade — UI/view only (it
//! READS the presenter's resolved events, and writes nothing back).
//!
//! This plugin is the `bevy_ui` HALF of the log only (GTW-620 C4): the spawn/despawn
//! lifecycle, the ONE appender, the animations, and the RON tuning. The sim-fact →
//! [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent) FORWARDERS (the
//! `CombatLogSource` impls, the generic forwarder, and the turn-boundary forwarder) live
//! in the presenter (`gdtf_battle_presenter`'s `actors/fx/fct/log_event/`), registered by
//! the renderer plugin beside the FCT drains over the same sim facts — so this plugin
//! imports NO sim fact message for logging, and adding a log source is a PRESENTER-ONLY
//! change.
//!
//! - **RON tuning** — the hot-reloadable [`CombatLogTuning`](super::tuning::CombatLogTuning)
//!   table registers through the GTW-564 generic hot-RON registration
//!   ([`register_combat_log_hot_ron`](super::tuning::register_combat_log_hot_ron)); it
//!   self-gates on an [`AssetServer`](bevy::asset::AssetServer), so a `MinimalPlugins`
//!   headless app skips it and the log runs on the defaults (`bevy-traps.md` #1).
//! - **Lifecycle** — [`spawn_combat_log`] `OnEnter(BattleScapeState::BattleRunning)`,
//!   [`despawn_combat_log`] `OnExit(BattleScapeState::BattleRunning)`.
//! - **The appender** — the ONE [`append_combat_log`] drains the presenter-owned
//!   [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent) buffer, classifies via the
//!   shared [`classify_log_event`](gdtf_battle_presenter::classify_log_event), spawns
//!   lines, and FIFO-trims. It orders itself strictly `.after` the presenter's exported
//!   [`CombatLogSystems::Forward`](gdtf_battle_presenter::CombatLogSystems) set — the
//!   cross-crate edge (`bevy-traps.md` #3) that keeps a sim fact written before an update
//!   a rendered line within that same update.
//! - **Animations** — [`fade_combat_log_lines`] / `slide` / `height` run unguarded (each
//!   self-gates on its entities existing) so a line spawned in the last `BattleRunning`
//!   frame still settles cleanly.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::*,
};
use gdtf_battle_presenter::{CombatLogEvent, CombatLogSystems};
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::combat_log::{
        systems::{
            animate_combat_log_height, append_combat_log, despawn_combat_log,
            fade_combat_log_lines, slide_combat_log_lines, spawn_combat_log,
        },
        tuning::register_combat_log_hot_ron,
    },
};

/// The combat-log scene-plugin — loads its hot-reloadable RON tuning, spawns/despawns the
/// log on the `BattleRunning` boundary, and wires the ONE appender after the presenter's
/// forwarders (GTW-620).
pub(in crate::states::running::game::battlescape) struct GameBattleScapeCombatLogScenePlugin;

impl Plugin for GameBattleScapeCombatLogScenePlugin {
    fn build(&self, app: &mut App) {
        // The hot-reloadable combat-log tuning registers through the GTW-564 generic
        // hot-RON registration — ONE ext call at its owning module, self-gated on the `AssetServer`
        // so a `MinimalPlugins` headless app skips it (`bevy-traps.md` #1).
        register_combat_log_hot_ron(app);
        add_systems(app);
    }
}

/// Register the combat-log lifecycle, the appender, and the animations.
fn add_systems(app: &mut App) {
    // The ONE appender: drain the resolved events, classify, spawn lines, FIFO-trim.
    // Strictly AFTER the presenter's forwarders via the exported CombatLogSystems::Forward
    // set (GTW-620 C3 — the explicit cross-crate ordering, bevy-traps.md #3), so a sim
    // fact written before an update is forwarded AND appended within that same update.
    // Gated on the live-battle witness + the presenter-owned Messages<CombatLogEvent>
    // buffer EXISTING (the presenter's registrar owns that add_message — the producer
    // registers its buffer): an app harness built without the renderer keeps the appender
    // inert instead of failing param validation (bevy-traps.md #1 / #4).
    app.add_systems(
        Update,
        append_combat_log.after(CombatLogSystems::Forward).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<Messages<CombatLogEvent>>),
        ),
    );

    app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_combat_log)
        .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_combat_log)
        // The fade + slide + height animations run UNGUARDED (each self-gates on its
        // entities existing) so a line spawned in the last BattleRunning frame still fades /
        // settles after the battle ends — the despawn on OnExit(BattleRunning) tears the
        // whole log down anyway. GTW-328 slice B: both slide + height read the prior
        // frame's `ComputedNode` layout (`ui_layout_system` runs in PostUpdate), a
        // one-frame lag that is imperceptible for a smooth lerp.
        .add_systems(
            Update,
            (
                fade_combat_log_lines,
                slide_combat_log_lines,
                animate_combat_log_height,
            ),
        );
}
