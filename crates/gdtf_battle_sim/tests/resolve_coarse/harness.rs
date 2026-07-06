//! Shared `resolve_coarse` fixture: the `setup_battle` app driver plus the
//! zero-cone / no-recoil shot-input builders shared by every test.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, Entity, MinimalPlugins},
    scene::ScenePlugin,
};
// The canonical shared builders + specs + registries + the `(cell, level)` key
// helper (consolidated out of this file's former local copies — GTW-324 migration).
use gdtf_battle_sim::test_support::{test_armor_registry, test_weapon_registry};
use gdtf_battle_sim::{
    cone::{ConeAngle, PriorShots},
    cover::HeightBand,
    ganger::Facing,
    prelude::{CellLevel, Direction, Position, Stance, StanceKind},
    resolve_coarse::ShotInputs,
    sample_cone::ConcentrationP,
    situation::{BattleRegistries, BattleSetup, Situation, setup_battle},
    stability::RecoilGrowth,
    tuning::{GangerStatTuning, RecoilClimb},
};

/// Run `setup_battle` on a fresh app, drive the `SpawnScene` schedule so the deferred
/// `bsn!` ganger components materialize (GTW-322), and return the app + `BattleSetup`
/// — or `None` on failure (keeping the tests free of `unwrap`/`expect`/`panic`, all
/// denied in tests too).
pub(crate) fn run_setup(situation: Situation) -> Option<(App, BattleSetup)> {
    // `AssetPlugin` + `ScenePlugin` are required: `setup_battle` now spawns each ganger
    // as a `bsn!` Scene (GTW-322), whose deferred materialization needs the scene/asset
    // infrastructure.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));

    let gangs = gdtf_battle_sim::test_support::test_gang_registry();
    let registry = test_weapon_registry();
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup.
    let melee = gdtf_battle_sim::test_support::test_melee_weapon_registry();
    let armor = test_armor_registry();
    let terrain = gdtf_battle_sim::test_support::test_terrain_registry();
    // GTW-384: setup derives each ganger's computed stats from the default stat tuning.
    let stat_tuning = GangerStatTuning::default();
    // GTW-396: fallback floor cost (no default_floor authored in test fixtures).
    let fallback_floor_cost = gdtf_battle_sim::tuning::CombatTuning::default()
        .move_costs
        .open;
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &registry,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                ),
                fallback_floor_cost,
                &mut commands,
            )
        });
    assert!(outcome.is_ok(), "the one-shot setup system must run");
    let setup = outcome.ok().and_then(Result::ok);
    assert!(setup.is_some(), "setup_battle must succeed on the fixture");
    let setup = setup?;
    // Only `app.update()` runs the `SpawnScene` schedule that turns the queued ganger
    // scenes into live components the per-entity `combat_snapshot` reads (GTW-322).
    app.update();
    Some((app, setup))
}

/// A zero cone angle — the sample is the aim axis EXACTLY (dead-center), so the
/// geometry is deterministic regardless of the RNG stream.
pub(crate) const fn zero_cone() -> ConeAngle {
    ConeAngle::new(0.0)
}

/// An arbitrary concentration exponent (NOT a shipped magnitude); irrelevant under
/// a zero cone but a real value for the seeded-replay test.
pub(crate) const fn some_p() -> ConcentrationP {
    ConcentrationP::new(2.0)
}

/// The first shot of an action with zero recoil climb / growth — the central axis
/// is the untilted muzzle→aim axis exactly.
pub(crate) const fn no_recoil() -> (PriorShots, RecoilClimb, RecoilGrowth) {
    (
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
}

/// The no-op corpse predicate (GTW-317) — marks no occupant dead, so every occupant
/// still stops the round. These `resolve_coarse` tests exercise the coarse pipeline
/// over live occupants only, so the dead-skip is never engaged here.
pub(crate) fn no_dead() -> impl Fn(Entity) -> bool {
    |_| false
}

/// Bundle the per-shot description into a [`ShotInputs`] (GTW-179): a standing East
/// shooter at `shooter_at` firing at a standing target at `target_at`, with the
/// given `cover_band` / cone width and zero recoil (the first shot). This is the
/// GTW-172 call surface, rewrapped — the geometry is identical.
pub(crate) const fn standing_shot(
    shooter_at: CellLevel,
    target_at: CellLevel,
    cover_band: Option<HeightBand>,
    cone: ConeAngle,
) -> ShotInputs {
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();
    ShotInputs {
        shooter_position: Position::new(shooter_at),
        shooter_facing: Facing::new(Direction::East),
        shooter_stance: Stance::new(StanceKind::Standing),
        target_position: Position::new(target_at),
        target_stance: Stance::new(StanceKind::Standing),
        cover_band,
        cone,
        p: some_p(),
        prior_shots,
        recoil_climb,
        recoil_growth,
    }
}
