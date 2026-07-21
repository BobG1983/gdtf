//! The thin app-side battle-lifecycle glue systems (E10.5 / GTW-207).
//!
//! These systems are the ONLY app-state-coupled part of the battle wiring: they name
//! the app's `BattleScapeState` / `GameState` (correct here — they are the app side)
//! and the Generation [`GenerationComplete`] marker, and they bridge them to the
//! SIM-OWNED lifecycle messages ([`SetupBattleRequested`] / [`BattleReady`] /
//! [`TeardownBattleRequested`]). The actual setup / teardown logic lives in
//! `gdtf_battle_sim::battle::BattleSimPlugin` — the sim owns its own integration; the app only
//! SENDS triggers and GATES on the sim's ready signal.
//!
//! Three systems, each in a different schedule slot:
//!
//! - [`request_battle_setup`] (`OnEnter(BattleScapeState::Generation)`): RUNS PROCGEN
//!   (GTW-433) — it generates the battle's terrain from the authored situation's
//!   `theme` + `grid_size` via the sim-owned
//!   [`generate_level`](gdtf_battle_sim::procgen::generate_level) (driven through
//!   [`procgen_battle_situation`](super::procgen::procgen_battle_situation)), merges that
//!   terrain over the authored gangers, and writes a [`SetupBattleRequested`] carrying the
//!   merged [`Situation`] + the per-battle [`BattleSeed`]. The authored situation is the
//!   persistent [`LoadedSituation`] clone if present, else [`Situation::default`].
//! - [`gate_generation_complete`] (`Update`, presence-gated): inserts
//!   [`GenerationComplete`] only on a [`BattleReady`] from the sim, so `move_on`
//!   advances strictly after a successful setup.
//! - [`request_battle_teardown`] (`OnExit(GameState::BattleScape)`): writes a
//!   [`TeardownBattleRequested`] so the sim cleans the battle-lifetime resources at
//!   the battle boundary (NOT the Generation sub-state boundary).

use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::ProcgenTuning,
    rng::BattleSeed,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};

use super::{
    procgen::{ProcgenRegistries, procgen_battle_situation},
    seed::resolve_root_seed,
};
use crate::states::{
    load::LoadedSituation, running::game::battlescape::generation::resources::GenerationComplete,
};

/// `OnEnter(BattleScapeState::Generation)`: PROCGEN the terrain, then trigger the sim to
/// build the battle (GTW-433).
///
/// Resolves the authored situation — the persistent [`LoadedSituation`] resolved by the
/// `Load` scene (E10.3) cloned by value if present, ELSE [`Situation::default()`] (an
/// EMPTY battlefield: zero gangers / links, validates trivially) — which now carries only
/// `theme` + `grid_size` + the placed gangers (GTW-433 C1: the authored situation no longer
/// holds inline terrain). It then RUNS PROCGEN via
/// [`procgen_battle_situation`](super::procgen::procgen_battle_situation): generate the
/// terrain from the authored `theme` ([`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid)) +
/// `grid_size` against the loaded UUID-keyed registries
/// ([`PrefabRegistry`] + [`UuidThemeRegistry`] + [`TerrainDefRegistry`], GTW-492) using a
/// [`ProcgenRng`](gdtf_battle_sim::rng::ProcgenRng) derived from
/// the resolved [`BattleSeed`], and merge that terrain over the authored gangers. The merged
/// [`Situation`] (`{authored gangers/spawn} + {procgen terrain}`) is written in a
/// [`SetupBattleRequested`] with the [`BattleSeed`] from [`resolve_root_seed`] (or a
/// pre-injected [`Res<BattleSeed>`] resource when present — the test-harness override that
/// guarantees replay determinism without touching wall-clock entropy). The sim's
/// `setup_battle_on_request` system (in `SimSystems::Simulate`) consumes the trigger, seeds
/// the five per-subsystem RNG streams (GTW-14: `ShotRng` / `SeverityRng` / `LootRng` /
/// `InjuryRng` / `ProcgenRng`), pours the situation into the world, and signals
/// [`BattleReady`] on success.
///
/// When any of the UUID-keyed registries is absent or procgen fails closed (e.g. an EMPTY
/// registry in a headless harness),
/// [`procgen_battle_situation`](super::procgen::procgen_battle_situation) returns the authored
/// situation UNCHANGED, so a battle with authored (or empty) terrain still sets up and
/// reaches `BattleRunning` (C4 — the existing state-walk keeps passing).
///
/// Seed resolution order:
/// 1. `Res<BattleSeed>` — pre-injected by a test harness (or a future seed-pick UI
///    feature) for deterministic replay; absent in the production GUI path.
/// 2. `resolve_root_seed()` — reads `GDTF_BATTLE_SEED` env var or falls back to the
///    wall-clock microsecond timestamp.
///
/// Whichever branch wins, the chosen seed is logged at `info!` UNCONDITIONALLY here
/// (GTW-14, m2) as the battle's replay handle — the override path skips
/// `resolve_root_seed` and the log it owns, so this site is the one guaranteed to
/// fire for every battle.
///
/// The `Situation::default()` fallback keeps the headless `MinimalPlugins` deep-walk
/// (which inserts no [`LoadedSituation`]) triggering a valid empty setup and
/// advancing rather than hanging in Generation (AC8).
#[expect(
    clippy::too_many_arguments,
    reason = "the params are the authored situation + seed override + the three UUID-keyed \
              procgen registries + the live tuning + the GTW-582 integrity report + the \
              setup writer — each a distinct Bevy SystemParam (Option<Res<_>> for the \
              Load-state resources); the sim's setup_battle_on_request precedent"
)]
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn request_battle_setup(
    loaded: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
    prefabs: Option<Res<PrefabRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    def_registry: Option<Res<TerrainDefRegistry>>,
    procgen_tuning: Option<Res<ProcgenTuning>>,
    report: Option<ResMut<ContentIntegrityReport>>,
    mut setup: MessageWriter<SetupBattleRequested>,
) {
    // The loaded authored battlefield if the Load scene resolved one, else the empty
    // Default (keeps the headless deep-walk green). Carries only theme + grid_size +
    // gangers now (GTW-433 C1: no inline terrain) — the terrain is GENERATED below.
    let authored: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    // GTW-14: use the pre-injected seed override (test harness) if present, else
    // resolve from the GDTF_BATTLE_SEED env var or wall-clock. The same injected,
    // deterministic per-battle seed drives procgen below (GTW-433 C2).
    let seed = seed_override.map_or_else(resolve_root_seed, |r| *r);
    // GTW-14 (m2): log the resolved seed UNCONDITIONALLY here so EVERY battle — both
    // the override path (test harness / future seed-pick UI, which bypasses
    // `resolve_root_seed` and the `info!` it owns) AND the entropy path — records its
    // replay handle. This is the one site guaranteed to fire for every setup.
    info!(
        seed = *seed,
        "battle setup: resolved BattleSeed (RNG replay handle)"
    );
    // GTW-433 / GTW-492: RUN PROCGEN. Generate the terrain from the authored theme
    // (ThemeUuid) + grid_size against the loaded UUID-keyed registries (PrefabRegistry +
    // UuidThemeRegistry + TerrainDefRegistry — populated by the GTW-489 Load resolve from the
    // GTW-490 migrated content, deterministic in `seed`) and merge it over the authored
    // gangers; falls back to the authored terrain when any registry is absent or procgen
    // fails closed (the headless empty-registry harness). This is the LIVE trigger that makes
    // procgen drive real battles. The procgen-emitted terrain references the migrated AUTHORED
    // terrain UUIDs, so `setup_battle` resolves it against the same `TerrainDefRegistry`
    // directly — the GTW-491 (T07a) `from_legacy_name` shim adapter is REMOVED.
    let registries = ProcgenRegistries {
        prefabs: prefabs.as_deref(),
        themes:  themes.as_deref(),
        terrain: def_registry.as_deref(),
        // GTW-533: the LIVE, hot-reloaded procgen fill tuning (the resident resource the
        // Load resolve inserts + re-reads on a `core_tuning/procgen.tuning.ron` edit); absent
        // only on the no-content headless harness, where procgen falls back to the default.
        tuning:  procgen_tuning.as_deref(),
    };
    let outcome = procgen_battle_situation(authored, registries, seed);
    // GTW-582 C3(d)/C5: every degraded resolution / last-resort fallback the
    // generation took was already warn!ed at its site; append it to the
    // persistent ContentIntegrityReport so the degradation is ON THE RECORD
    // beside the end-of-Load findings. `Option` because a bespoke harness may
    // not install the Load plugin's report (bevy-traps #1); the real app always
    // carries it.
    if let Some(mut report) = report {
        for finding in outcome.findings {
            report.record(finding);
        }
    }
    // GTW-744: FAIL CLOSED on a typed roster-deployment failure — write NO SetupBattleRequested,
    // so the sim never signals BattleReady and `gate_generation_complete` never inserts the
    // marker: the machine stays in Generation rather than starting an under-populated battle
    // (the same no-BattleReady contract a bad situation already relies on).
    if let Some(err) = &outcome.deployment_error {
        error!(
            "procgen could not deploy the roster into its deployment zone ({err}); the battle \
             will not set up (staying in Generation)"
        );
        return;
    }
    setup.write(SetupBattleRequested::new(outcome.situation, seed));
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
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn gate_generation_complete(
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
/// resources ([`ShotRng`](gdtf_battle_sim::rng::ShotRng) + the four grids) MUST SURVIVE
/// `Generation` → `AnimateIn` → `BattleRunning` → `AnimateOut` → `AfterMath` because
/// the E10.6 acts consume them in `BattleRunning`, so the sim cleans them only when
/// the whole battle ends (`bevy-traps.md` #1 applied at the CORRECT — battle — level).
///
/// [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) is untouched by the sim's
/// teardown: it is E10.4's persistent `Load` resource. The [`GenerationComplete`]
/// marker is Generation-scoped and cleaned by the generation `cleanup` system on
/// `OnExit(Generation)`, not here.
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn request_battle_teardown(
    mut teardown: MessageWriter<TeardownBattleRequested>,
) {
    teardown.write(TeardownBattleRequested);
}
