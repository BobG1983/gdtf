//! The two-ganger situation, the driven battle app, and shared state readers.

use bevy::state::state::State;
use gdtf_app::test_support::BattleScapeState;
use gdtf_battle_sim::{
    ganger::{Direction, Facing, Faction},
    metric::CellLevel,
    situation::{GangerSpawn, Situation},
    test_support::{GangerSpawnBuilder, SituationBuilder, key},
};
use gdtf_test_utils::BattleAppBuilder;

/// A budget large enough to advance the live battle through the GTW-236/GTW-239
/// lifecycle assertions, but bounded so a machine that never reaches the predicate fails
/// instead of hanging. (The descent to `BattleRunning` is the shared
/// [`BattleAppBuilder`]'s concern; this budget covers only the per-test advances.)
pub(crate) const BUDGET: u32 = 96;

/// The shooter's faction in the drive-proof fixture (the armed ganger).
pub(crate) const SHOOTER_FACTION: u8 = 0;
/// The target's faction in the drive-proof fixture (the ganger fired at).
pub(crate) const TARGET_FACTION: u8 = 1;

/// The `(cell, level)` the shooter is authored at — west of the target, same storey,
/// so a due-East shot reaches it.
pub(crate) const SHOOTER_AT: (i32, i32, u8) = (2, 5, 0);
/// The `(cell, level)` the target is authored at — due East of the shooter.
pub(crate) const TARGET_AT: (i32, i32, u8) = (8, 5, 0);

/// Build the drive-proof fixture's authored ganger at `at` / `faction` via the central
/// [`GangerSpawnBuilder`](gdtf_battle_sim::test_support::GangerSpawnBuilder): facing East
/// (so a due-East shot reaches the target), referencing the central test weapon + armor
/// keys (the builder default), so a setup arms + armors it from the registries the
/// [`BattleAppBuilder`] seeds.
pub(crate) fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::East))
        .build()
}

/// A valid two-ganger fixture situation (link-free → validates trivially): an armed-able
/// shooter (faction `SHOOTER_FACTION`) facing East, and a target (faction `TARGET_FACTION`)
/// directly East, built over the central
/// [`SituationBuilder`](gdtf_battle_sim::test_support::SituationBuilder). The
/// `SetupBattleRequested` the app sends on `OnEnter(Generation)` pours this real battle
/// into the world before `BattleRunning` (RNG streams / `CombatTuning` / `OccupancyGrid`
/// present, both gangers spawned by `setup_battle` via `Commands`).
pub(crate) fn two_ganger_situation() -> Situation {
    let (sx, sy, sl) = SHOOTER_AT;
    let (tx, ty, tl) = TARGET_AT;
    SituationBuilder::new()
        .with_gangers([
            ganger_at(key(sx, sy, sl), SHOOTER_FACTION),
            ganger_at(key(tx, ty, tl), TARGET_FACTION),
        ])
        .build()
}

/// Reads the current [`BattleScapeState`] if it is active.
pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Builds the headless app already driven to a live battle via the shared
/// [`BattleAppBuilder`], seeded with the drive-proof [`two_ganger_situation`] and the
/// canonical `Load` resources (theme / tuning / test weapon + armor registries) the
/// builder injects. Returns `None` if the shared drive does not reach `BattleRunning`
/// (the caller asserts the `Some`).
pub(crate) fn driven_battle_app() -> Option<bevy::app::App> {
    BattleAppBuilder::new()
        .with_situation(two_ganger_situation())
        .build()
}

/// Whether [`State<BattleScapeState>`] has reached or passed `AnimateOut` (it is no longer in
/// `BattleRunning`). `AnimateOut` is the immediate successor of `BattleRunning`; on a slow
/// machine an `advance_until` predicate keyed purely on `AnimateOut` could miss it if the state
/// kept advancing, so the win/loss end tests below assert `AnimateOut` directly after a bounded
/// drive rather than rely on equality alone.
pub(crate) fn left_battle_running(app: &bevy::app::App) -> bool {
    battlescape_state(app) != Some(BattleScapeState::BattleRunning)
}
