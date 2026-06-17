//! Shared test fixtures + the crate-symbol re-exports for the `acts` dispatch tests.
//!
//! The concern test files (`request` / `plugin` / `fire` / `posture` / `downed` /
//! `movement` / `coschedule`) each glob `use super::support::*` to reach these helpers
//! plus the crate types they exercise — the single glob the flat module's combined
//! `use super::*` and `use crate::{…}` block used to provide before the GTW-201 dir-split.
//! The fixtures are RELOCATED VERBATIM (same construction, same magnitudes); no test logic
//! changed.

// Re-exported so each concern file's `use super::support::*` reaches the Bevy harness
// types + every `acts` dispatch/message item + the crate components the tests touch.
pub(super) use bevy::prelude::{App, Entity, MinimalPlugins, Update, World};

pub(super) use crate::{
    acts::*,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        SourceArmor, WornArmor,
    },
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stabilized,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    rng::{BattleSeed, SimRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

/// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Insert the shared sim resources a dispatch system reads — the three grids, a
/// seeded [`SimRng`], and [`CombatTuning::default`]. The grids are inserted EMPTY by
/// default; a test mutates them via `app.world_mut()` before the run.
pub(super) fn insert_sim_resources(app: &mut App) {
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app.insert_resource(SimRng::from_seed(BattleSeed::new(SEED)));
    app.insert_resource(CombatTuning::default());
}

/// Build a headless app: [`MinimalPlugins`] (no window / renderer) + [`SimActsPlugin`]
/// + the shared sim resources — the `apply_hit` / `occupancy_sync` test precedent.
pub(super) fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app);
    app
}

/// A single-shot fire-mode spec from arbitrary (non-pinned) per-mode numbers.
pub(super) const fn single_mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A worn suit whose every piece starts at the given stats — arbitrary (not shipped)
/// magnitudes so a hit lands in a known regime.
pub(super) fn worn_suit(floor: i32, protection: i32, integrity: i32, hardness: i32) -> WornArmor {
    WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(floor),
        ArmorProtection::new(protection),
        ArmorIntegrity::new(integrity),
        ArmorHardness::new(hardness),
        ArmorType::DEFAULT,
    )))
}

/// Spawn an armed shooter facing East at `(x, y, 0)` — carries the full shooter-query
/// component set AND the target-query set (the shooter is also a ganger, so its own
/// liveness reads from the target query). Arbitrary magnitudes (not shipped tuning).
pub(super) fn spawn_shooter(
    world: &mut World,
    x: i32,
    y: i32,
    mode: FireModeSpec,
    aiming: bool,
) -> Entity {
    let mag_size = MagazineSize::new(30);
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(mag_size, FireMode::new(vec![mode]), Stable::new(true)),
    );
    world
        .spawn((
            bundle,
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(aiming),
            Shooting::new(1.0),
            Tu::new(200),
            crate::ganger::TuMax::new(100),
            Magazine::new(10, mag_size),
            // The target-query set as one nested-tuple bundle (the GTW-279
            // InflictedWounds add keeps the spawn under the 15-element tuple limit).
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                worn_suit(0, 0, 1, 0),
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id()
}

/// The per-ganger battle-state bundle a fire target carries (the target query set +
/// worn armor) — arbitrary magnitudes.
pub(super) fn target_bundle(hp: u16, wounds: u8, worn: WornArmor) -> impl bevy::prelude::Bundle {
    (
        Hp::new(hp),
        Wounds::new(wounds),
        LifeState::Alive,
        worn,
        InflictedWounds::default(),
        Toughness::new(1.0),
        Luck::new(0.0),
    )
}

/// Build the FIRE scenario in a fresh app: a shooter at (2,5) + a HIGH-band ganger
/// target directly East at (8,5,0) in the occupancy grid. Returns `(app, shooter,
/// target)`. Mirrors the fire.rs in-line target setup.
pub(super) fn fire_scenario() -> (App, Entity, Entity) {
    let mut app = headless_app();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
    let target = app
        .world_mut()
        .spawn(target_bundle(30, 6, worn_suit(0, 0, 1, 0)))
        .id();
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(target_at, Some(target));
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }
    (app, shooter, target)
}

/// Spawn a downed-act actor at `(x, y, 0)` of `faction`, [`LifeState::Alive`].
pub(super) fn spawn_downed_actor(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            LifeState::Alive,
            Faction::new(faction),
            Stabilized::new(false),
        ))
        .id()
}

/// Spawn a downed-act target at `(x, y, 0)` of `faction`, [`LifeState::Downed`], not
/// yet stabilized.
pub(super) fn spawn_downed_target(world: &mut World, x: i32, y: i32, faction: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            LifeState::Downed,
            Faction::new(faction),
            Stabilized::new(false),
        ))
        .id()
}
