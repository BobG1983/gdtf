//! The thin app-side battle-lifecycle glue systems (E10.5 / GTW-207).
//!
//! These systems are the ONLY app-state-coupled part of the battle wiring: they name
//! the app's `BattleScapeState` / `GameState` (correct here — they are the app side)
//! and the Generation [`GenerationComplete`] marker, and they bridge them to the
//! SIM-OWNED lifecycle messages ([`SetupBattleRequested`] / [`BattleReady`] /
//! [`TeardownBattleRequested`]). The actual setup / teardown logic lives in
//! `gdtf_battle_sim::BattleSimPlugin` — the sim owns its own integration; the app only
//! SENDS triggers and GATES on the sim's ready signal.
//!
//! Three systems, each in a different schedule slot:
//!
//! - [`request_battle_setup`] (`OnEnter(BattleScapeState::Generation)`): writes a
//!   [`SetupBattleRequested`] carrying the authored [`Situation`] (the persistent
//!   [`LoadedSituation`] clone if present, else [`Situation::default`]) + a fixed
//!   placeholder [`BattleSeed`].
//! - [`gate_generation_complete`] (`Update`, presence-gated): inserts
//!   [`GenerationComplete`] only on a [`BattleReady`] from the sim, so `move_on`
//!   advances strictly after a successful setup.
//! - [`request_battle_teardown`] (`OnExit(GameState::BattleScape)`): writes a
//!   [`TeardownBattleRequested`] so the sim cleans the battle-lifetime resources at
//!   the battle boundary (NOT the Generation sub-state boundary).

use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
    rng::BattleSeed,
    situation::Situation,
};

use crate::scenes::{
    load::LoadedSituation, running::game::battlescape::generation::resources::GenerationComplete,
};

/// The fixed battle seed this slice threads into the setup trigger.
///
/// A documented PLACEHOLDER: no E10 slice yet lands a composition-root seed resource
/// (E10.0 builds only the `SimSystems` set; nothing inserts a chosen seed for
/// Generation to read), so the app — the composition root — owns producing one here.
/// The menu / setup-scene seed-pick (`GDTF_BATTLE_SEED` per
/// `docs/combat/resolution.md`, GTW-14's roster-side producer) is a LATER slice that
/// will replace this constant with the player-chosen / env-pinned seed. It reuses the
/// [`BattleSeed`](gdtf_battle_sim::rng::BattleSeed) newtype (no new bare domain
/// primitive — `.claude/rules/no-bare-types.md`).
const DEFAULT_BATTLE_SEED: BattleSeed = BattleSeed::new(0);

/// `OnEnter(BattleScapeState::Generation)`: trigger the sim to build the battle.
///
/// Resolves the situation — the persistent [`LoadedSituation`] resolved by the
/// `Load` scene (E10.3) cloned by value if present, ELSE [`Situation::default()`] (an
/// EMPTY battlefield: zero gangers / links, validates trivially) — and writes a
/// [`SetupBattleRequested`] carrying it + [`DEFAULT_BATTLE_SEED`]. The sim's
/// `setup_battle_on_request` system (in `SimSystems::Simulate`) consumes the trigger,
/// seeds the [`SimRng`](gdtf_battle_sim::rng::SimRng), pours the situation into the
/// world, and signals [`BattleReady`] on success.
///
/// The `Situation::default()` fallback keeps the headless `MinimalPlugins` deep-walk
/// (which inserts no [`LoadedSituation`]) triggering a valid empty setup and
/// advancing rather than hanging in Generation (AC8).
pub(in crate::scenes::running::game::battlescape::generation::battle_sim) fn request_battle_setup(
    loaded: Option<Res<LoadedSituation>>,
    mut setup: MessageWriter<SetupBattleRequested>,
) {
    // The loaded authored battlefield if the Load scene resolved one, else the empty
    // Default (keeps the headless deep-walk green).
    let situation: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    setup.write(SetupBattleRequested::new(situation, DEFAULT_BATTLE_SEED));
}

/// `Update` (presence-gated): insert [`GenerationComplete`] on a [`BattleReady`] from
/// the sim.
///
/// Replaces the previous unconditional no-op insert. It runs only while
/// `in_state(Generation)` AND [`GenerationComplete`] is absent (the run-condition is
/// wired in the plugin), and inserts the marker ONLY when the sim has signalled
/// [`BattleReady`] — the setup-complete signal `setup_battle_on_request` writes on
/// success and NEVER on a failed / absent setup. So a bad situation keeps the machine
/// in Generation (the existing `move_on`, gated on the marker, advances strictly after
/// — `bevy-traps.md` #3). Reading the signal (rather than polling a resource the setup
/// happens to insert) keeps the gate keyed on the sim's explicit ready contract.
pub(in crate::scenes::running::game::battlescape::generation::battle_sim) fn gate_generation_complete(
    mut ready: MessageReader<BattleReady>,
    mut commands: Commands,
) {
    // Any BattleReady this run means the sim set the battle up successfully — drain
    // the buffer and insert the marker (the run-condition gates re-entry, so a single
    // insert is enough; draining clears the signal either way).
    if ready.read().next().is_some() {
        commands.insert_resource(GenerationComplete);
    }
}

/// `OnExit(GameState::BattleScape)`: trigger the sim to tear the battle down.
///
/// Writes a [`TeardownBattleRequested`] at the BATTLE boundary, NOT the
/// `OnExit(BattleScapeState::Generation)` boundary: the sim's battle-lifetime
/// resources ([`SimRng`](gdtf_battle_sim::rng::SimRng) + the four grids) MUST SURVIVE
/// `Generation` → `AnimateIn` → `BattleRunning` → `AnimateOut` → `AfterMath` because
/// the E10.6 acts consume them in `BattleRunning`, so the sim cleans them only when
/// the whole battle ends (`bevy-traps.md` #1 applied at the CORRECT — battle — level).
///
/// [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) is untouched by the sim's
/// teardown: it is E10.4's persistent `Load` resource. The [`GenerationComplete`]
/// marker is Generation-scoped and cleaned by the generation `cleanup` system on
/// `OnExit(Generation)`, not here.
pub(in crate::scenes::running::game::battlescape::generation::battle_sim) fn request_battle_teardown(
    mut teardown: MessageWriter<TeardownBattleRequested>,
) {
    teardown.write(TeardownBattleRequested);
}
