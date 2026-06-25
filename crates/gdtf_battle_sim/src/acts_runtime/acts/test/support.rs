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
pub(super) use bevy::{
    platform::collections::HashSet,
    prelude::{App, Entity, Messages, MinimalPlugins, Update, World},
};

pub(super) use crate::{
    acts::*,
    // `ArmorProtection` / `ArmorHardness` are reused for cover `CoverEntry`s in the
    // co-schedule test (terrain armor), not ganger-worn armor — kept for that glob use.
    armor::{ArmorHardness, ArmorProtection},
    battle::PlayerFaction,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stabilized,
        Stance, StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, ReloadTu, mode_tu_cost},
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    occupancy_sync::OccupancyMaintenancePlugin,
    resolve_coarse::ShotKind,
    rng::{BattleSeed, InjuryRng, LootRng, ProcgenRng, SeverityRng, ShotRng},
    shot_fired::ShotFired,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy,
        Wields,
    },
};

/// A fixed seed for the per-test RNG stream (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// The player gang the move-dispatch harness seeds (arbitrary; gang `0`).
pub(super) const TEST_PLAYER_GANG: u8 = 0;

/// A [`SquadVisibility`] with the ENTIRE grid extent both VISIBLE and EXPLORED — the
/// "full vision" fog the GTW-354 move-dispatch tests route under (every cell routable, so
/// the GTW-353 visibility gate is a no-op and the route depends only on geometry +
/// occupancy). Mirrors the pathfinder-test `full_vision` fixture.
pub(super) fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "x/y are 0..60 and level is 0..MAX_LEVELS (8) by the loop bounds, so \
                              the usize/u8 -> i32/u8 narrowing cannot truncate or wrap"
                )]
                let c = CellLevel::new(Cell::new(x as i32, y as i32), Level::new(level));
                all.insert(c);
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

/// Insert the shared sim resources a dispatch system reads — the three grids, the empty
/// [`VerticalLinkGraph`], a full-vision [`SquadVisibility`], the [`PlayerFaction`] seed,
/// the five seeded per-subsystem RNG streams (GTW-14), [`CombatTuning::default`], and the [`FloorCostGrid`] (GTW-396
/// Decision B — seeded uniform at the default open cost so the adjacent-cell move tests
/// keep their existing cost semantics). The grids are inserted EMPTY by default; a test
/// mutates them via `app.world_mut()` before the run.
///
/// GTW-354: the move dispatch now reads `Res<VerticalLinkGraph>` / `Res<SquadVisibility>`
/// / `Res<PlayerFaction>` for its route gate, so the harness seeds them — a full-vision
/// fog and an empty link graph keep the planar adjacent-cell move tests routing on
/// geometry/occupancy alone.
pub(super) fn insert_sim_resources(app: &mut App) {
    let tuning = CombatTuning::default();
    // GTW-396: seed a uniform FloorCostGrid at the default open cost; the test harness
    // does not have a TerrainRegistry, so we build the grid directly with the same move
    // cost the pre-GTW-396 tests expected. The dispatch system reads `Res<FloorCostGrid>`.
    let floor_costs = FloorCostGrid::new(tuning.move_costs.open, []);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    // GTW-365: `dispatch_fire` reads `ResMut<SlabLedger>` — seed an empty ledger.
    app.insert_resource(SlabLedger::new());
    // GTW-392: `dispatch_fire` reads `Res<BraceStairCells>` — seed an empty set (no
    // stair links in the test arena, so no brace-stair cells are authored).
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(full_vision());
    app.insert_resource(PlayerFaction::new(Faction::new(TEST_PLAYER_GANG)));
    // GTW-14: insert the five per-subsystem RNG streams derived from the test seed.
    let seed = BattleSeed::new(SEED);
    app.insert_resource(ShotRng::from_root(seed));
    app.insert_resource(SeverityRng::from_root(seed));
    app.insert_resource(LootRng::from_root(seed));
    app.insert_resource(InjuryRng::from_root(seed));
    app.insert_resource(ProcgenRng::from_root(seed));
    app.insert_resource(tuning);
    app.insert_resource(floor_costs);
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

/// The per-ganger battle-state bundle a fire target carries (the target query set) —
/// arbitrary magnitudes. Since GTW-323 the armor lives on related piece entities, NOT
/// the ganger, so this bundle carries no armor.
pub(super) fn target_bundle(hp: u16, wounds: u8) -> impl bevy::prelude::Bundle {
    (
        Hp::new(hp),
        Wounds::new(wounds),
        LifeState::Alive,
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
