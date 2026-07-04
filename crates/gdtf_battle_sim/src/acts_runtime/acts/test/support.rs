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
pub(super) use bevy::prelude::{App, Entity, Messages, Update, World};

// The canonical shared harness/fixture halves (GTW-576): the ONE seeding litany +
// the knobbed app builder + the fire-mode/target fixtures, consolidated out of this
// file's former local copies.
pub(super) use crate::test_support::{SimAppBuilder, TEST_PLAYER_GANG, single_mode, target_bundle};
pub(super) use crate::{
    acts::*,
    // `ArmorProtection` / `ArmorHardness` are reused for cover `CoverEntry`s in the
    // co-schedule test (terrain armor), not ganger-worn armor — kept for that glob use.
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stabilized,
        Stance, StanceKind, Suppressed, SuppressorCell, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    resolve_and_apply::HitVerdict,
    resolve_coarse::ShotKind,
    shot_fired::ShotFired,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy, Wields,
    },
};

/// Build a headless app: [`MinimalPlugins`] (no window / renderer) + [`SimActsPlugin`] +
/// the canonical sim-resource litany with a FULL-VISION fog (the GTW-354 move-dispatch
/// precondition) — this module's bespoke composition of the shared GTW-576 builder.
pub(super) fn headless_app() -> App {
    SimAppBuilder::new().with_acts().with_full_vision().build()
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
    let reload_tu = ReloadTu::new(12);
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
        // A known 10-round load (the fire tests assert a fixed starting count); the
        // bundle carries this Magazine directly (GTW-275: WeaponBundle::new uses the
        // handling's magazine as-authored, so no separate Magazine insert is needed —
        // a second Magazine in the spawn tuple would be a duplicate-component panic).
        HandlingProfile::new(
            Magazine::new(10, mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(aiming),
            Shooting::new(1.0),
            Tu::new(200),
            crate::ganger::TuMax::new(100),
            // The target-query set as one nested-tuple bundle (the GTW-279
            // InflictedWounds add); the shooter's armor (if any) lives on related piece
            // entities (GTW-323), NOT here.
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`), read by
    // `dispatch_fire`/`fire()` + `dispatch_reload` through `ganger → Wields → the weapon
    // entity`. The `WieldedBy` insert hook populates the ganger's `Wields` synchronously
    // in a bare `World` spawn so the very next dispatch resolves it.
    world.spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// Build the FIRE scenario in a fresh app: a shooter at (2,5) + a HIGH-band ganger
/// target directly East at (8,5,0) in the occupancy grid. Returns `(app, shooter,
/// target)`. Mirrors the fire.rs in-line target setup.
pub(super) fn fire_scenario() -> (App, Entity, Entity) {
    let mut app = headless_app();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(app.world_mut(), 2, 5, mode, true);
    let target = app.world_mut().spawn(target_bundle(30, 6)).id();
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
