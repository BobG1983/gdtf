//! The combat-log scene-plugin (GTW-328, slice 3, bottom-left, ABOVE the weapon panel;
//! GTW-572 rebuilds the drain as the forwarder → appender message seam).
//!
//! Registers the battle-scoped combat-text LOG in the battlescape neighborhood. The log
//! shows the most-recent combat events as lines that scroll up and fade — UI/view only (it
//! READS the sim's fact messages + the ganger names, and writes nothing back).
//!
//! - **RON tuning** — the hot-reloadable [`CombatLogTuning`](super::tuning::CombatLogTuning)
//!   table registers through the GTW-564 generic hot-RON seam
//!   ([`register_combat_log_hot_ron`](super::tuning::register_combat_log_hot_ron)); it
//!   self-gates on an [`AssetServer`](bevy::asset::AssetServer), so a `MinimalPlugins`
//!   headless app skips it and the log runs on the defaults (`bevy-traps.md` #1).
//! - **Lifecycle** — [`spawn_combat_log`] `OnEnter(BattleScapeState::BattleRunning)`,
//!   [`despawn_combat_log`] `OnExit(BattleScapeState::BattleRunning)`.
//! - **The GTW-572 C5 seam** — the plugin owns the buffered
//!   [`CombatLogEvent`](gdtf_battle_presenter::CombatLogEvent) message
//!   (`add_message` here — the seam's owner, the `HighlightRequest` precedent) and the
//!   explicit `Forward → Append` set chain (`bevy-traps.md` #3). Each log SOURCE is one
//!   [`add_combat_log_source`](CombatLogSourceAppExt::add_combat_log_source) line (the thin
//!   generic forwarder, gated on the live-battle witness + the source's own buffer — the
//!   registrar never `add_message`s a source buffer: the sim plugins / the presenter
//!   renderer register those); the turn boundary rides the one bespoke
//!   [`forward_turn_started`] (it needs `PlayerFaction`). The ONE
//!   [`append_combat_log`] appender drains the events, classifies via the shared
//!   [`classify_log_event`](gdtf_battle_presenter::classify_log_event), spawns lines, and
//!   FIFO-trims. Adding a log source = one forwarder impl + one registrar line (+ one
//!   classify arm if the phrasing is new).
//! - **Animations** — [`fade_combat_log_lines`] / `slide` / `height` run unguarded (each
//!   self-gates on its entities existing) so a line spawned in the last `BattleRunning`
//!   frame still settles cleanly.

use bevy::{ecs::schedule::SystemCondition, prelude::*};
use gdtf_battle_presenter::{CombatLogEvent, ShotImpactResolved};
use gdtf_battle_sim::{
    ArmorBroken, BattleInProgress, BleedStarted, DotAfflicted, FallOccurred, FieldAfflicted,
    FireDeclaration, InjuryInflicted, MeleeStruck, MoveRejected, MovementOccurred, OnDeathOccurred,
    ReloadResult, SuppressionApplied, TurnStarted,
};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::combat_log::{
        systems::{
            CombatLogSourceAppExt, CombatLogSystems, animate_combat_log_height, append_combat_log,
            despawn_combat_log, fade_combat_log_lines, forward_turn_started,
            slide_combat_log_lines, spawn_combat_log,
        },
        tuning::register_combat_log_hot_ron,
    },
};

/// The combat-log scene-plugin — loads its hot-reloadable RON tuning, spawns/despawns the
/// log on the `BattleRunning` boundary, and wires the GTW-572 forwarder → appender seam.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeCombatLogScenePlugin;

impl Plugin for GameBattleScapeCombatLogScenePlugin {
    fn build(&self, app: &mut App) {
        // The hot-reloadable combat-log tuning registers through the GTW-564 generic
        // hot-RON seam — ONE ext call at its owning module, self-gated on the `AssetServer`
        // so a `MinimalPlugins` headless app skips it (`bevy-traps.md` #1).
        register_combat_log_hot_ron(app);
        add_systems(app);
    }
}

/// Register the combat-log lifecycle, the forwarder → appender seam, and the animations.
fn add_systems(app: &mut App) {
    // GTW-572 C5: the plugin OWNS the resolved-event seam — the CombatLogEvent buffer is
    // registered unconditionally here (the seam's owner; every forwarder's MessageWriter
    // and the appender's MessageReader need it — bevy-traps.md #4). Source buffers are NOT
    // registered here: each forwarder is gated on its source's Messages<S> existing, so in
    // a live battle the sim plugins / presenter renderer provide them and a focused harness
    // that omits one simply keeps that forwarder inert.
    app.add_message::<CombatLogEvent>();
    // The explicit Forward → Append chain (bevy-traps.md #3): a sim fact written before an
    // update is forwarded AND appended within that same update.
    app.configure_sets(
        Update,
        (CombatLogSystems::Forward, CombatLogSystems::Append).chain(),
    );

    // The log sources — one registrar line each (GTW-572 C5; adding a source = one
    // CombatLogSource impl in systems/sources.rs + one line here). The GTW-328 combat
    // events first, then the GTW-572 C6 state changes (the Q2 ruling: falls, melee damage,
    // on-death kills, suppression, armor-broken, and the three once-at-start afflictions —
    // their per-tick signals have no source impl at all).
    app.add_combat_log_source::<FireDeclaration>()
        .add_combat_log_source::<MovementOccurred>()
        .add_combat_log_source::<MoveRejected>()
        .add_combat_log_source::<ShotImpactResolved>()
        .add_combat_log_source::<ReloadResult>()
        .add_combat_log_source::<InjuryInflicted>()
        .add_combat_log_source::<FallOccurred>()
        .add_combat_log_source::<MeleeStruck>()
        .add_combat_log_source::<OnDeathOccurred>()
        .add_combat_log_source::<SuppressionApplied>()
        .add_combat_log_source::<ArmorBroken>()
        .add_combat_log_source::<DotAfflicted>()
        .add_combat_log_source::<FieldAfflicted>()
        .add_combat_log_source::<BleedStarted>();
    // The turn boundary is the one bespoke forwarder (it reads PlayerFaction to label
    // Player vs Enemy) — same set, same gates.
    app.add_systems(
        Update,
        forward_turn_started
            .in_set(CombatLogSystems::Forward)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<bevy::ecs::message::Messages<TurnStarted>>),
            ),
    );

    // The ONE appender: drain the resolved events, classify, spawn lines, FIFO-trim.
    app.add_systems(
        Update,
        append_combat_log
            .in_set(CombatLogSystems::Append)
            .run_if(resource_exists::<BattleInProgress>),
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
