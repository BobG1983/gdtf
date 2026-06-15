//! [`BattleSimPlugin`] — the thin app-side glue that drives the SIM-OWNED
//! `gdtf_battle_sim::BattleSimPlugin` across the battle lifecycle (E10.5 / GTW-207).
//!
//! It is the VIEW-side seam (`docs/decisions/0001-rust-bevy-rewrite.md`: the model is
//! the authoritative render-free sim, consumed ONE-WAY by the app). The sim owns its
//! own integration plugin and names NO app type; this app-side plugin adds that sim
//! plugin (one line) and the THIN trigger / gate / teardown systems that bridge the
//! app's state machine to the sim's lifecycle messages — it only SENDS triggers and
//! GATES on the sim's [`BattleReady`](gdtf_battle_sim::battle::BattleReady) signal,
//! never duplicating combat rules. Registered by the
//! [`GameBattleScapeGenerationScenePlugin`](super::super::GameBattleScapeGenerationScenePlugin)
//! tree; it adds no new Cargo edge (the `gdtf_app -> gdtf_battle_sim` path is E10.1's,
//! consumed here).

use bevy::prelude::*;
use gdtf_battle_sim::{BattleSimPlugin as SimBattleSimPlugin, occupancy_sync::SimSystems};

use crate::{
    scenes::running::game::battlescape::generation::{
        battle_sim::systems::{
            gate_generation_complete, request_battle_setup, request_battle_teardown,
        },
        resources::GenerationComplete,
    },
    states::{BattleScapeState, GameState},
};

/// Drives the sim-owned `gdtf_battle_sim::BattleSimPlugin` across the app's battle
/// lifecycle.
///
/// Wiring (all additive — it touches no presenter / camera / window / input):
///
/// 1. Adds [`gdtf_battle_sim::BattleSimPlugin`](SimBattleSimPlugin) — the ONE plugin
///    that wires the whole sim runtime (bundling `OccupancyMaintenancePlugin` +
///    `SimActsPlugin`, registering the three lifecycle messages, and adding the sim's
///    setup / teardown systems in `SimSystems::Simulate`).
/// 2. `OnEnter(BattleScapeState::Generation)`:
///    [`request_battle_setup`](super::systems::request_battle_setup) writes a
///    `SetupBattleRequested` carrying the authored situation + the placeholder seed.
/// 3. `Update`, presence-gated and ordered `.after(SimSystems::Simulate)`:
///    [`gate_generation_complete`](super::systems::gate_generation_complete) inserts
///    [`GenerationComplete`] on the sim's `BattleReady` signal — strictly after the
///    sim's setup system has run THIS update (`bevy-traps.md` #3), so the signal
///    written this update is read this update (no one-frame lag), and the existing
///    `move_on` advances to `AnimateIn` only after a successful setup.
/// 4. `OnExit(GameState::BattleScape)`:
///    [`request_battle_teardown`](super::systems::request_battle_teardown) writes a
///    `TeardownBattleRequested` so the sim cleans the battle-lifetime resources at the
///    BATTLE boundary (NOT the Generation boundary) — they survive the whole battle
///    for the E10.6 acts (`bevy-traps.md` #1 at the correct state level).
pub(in crate::scenes::running::game::battlescape::generation) struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimBattleSimPlugin)
            .add_systems(OnEnter(BattleScapeState::Generation), request_battle_setup)
            .add_systems(
                Update,
                gate_generation_complete.after(SimSystems::Simulate).run_if(
                    in_state(BattleScapeState::Generation)
                        .and(not(resource_exists::<GenerationComplete>)),
                ),
            )
            .add_systems(OnExit(GameState::BattleScape), request_battle_teardown);
    }
}
