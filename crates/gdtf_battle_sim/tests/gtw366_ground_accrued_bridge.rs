//! GTW-366 C7 — the ground fire→`GroundAccrued`→`accrue_ground_damage` bridge drives the
//! REAL downstream wiring.
//!
//! A round that exits the bottom of the voxel column strikes the GROUND — damaged, never
//! destroyed — and makes the WIRED `dispatch_fire` emit a buffered `GroundAccrued`, which
//! the WIRED `sync_accrued_ground` accrues onto the `SurfaceGrid`'s per-cell ground
//! accumulator. This test drives the ACTUAL `dispatch_fire` + `sync_accrued_ground` systems
//! (NOT a reimplementation): it builds a real headless app with the production plugins,
//! fires a `FireRequested` straight DOWN through an open column at the ground, and asserts:
//!
//! - **C7(a)** a ground strike ACCRUES the round's `weapon_damage` onto the CORRECT cell's
//!   accumulator (the ground-plane cell the round exits through), proved via the live
//!   `SurfaceGrid::ground_damage` read after one fired round;
//! - **C7(b)** MONOTONIC — repeated strikes SUM and the accumulator NEVER decreases (a
//!   strictly-growing total across several fired rounds, each adding the same weapon damage);
//! - **C7(c)** NO ganger / cover / slab state is mutated by a ground strike — the shooter's
//!   HP/Wounds and the cover/slab ledgers + slab grid state read EXACTLY as seeded after the
//!   barrage (purely cosmetic accrual).
//!
//! NO shipped magnitudes are pinned — the assertions are relations (accrued == sum of the
//! per-round weapon damage, the total strictly grows, the other state is unchanged). The
//! ground path takes NO RNG draw, so it is replay-deterministic by construction (the §"What's
//! pure math vs sim" property). Render-free, zero pixels; the one `World` mutation is in a
//! TEST BODY (`bevy-traps.md` #7 carve-out) — no helper here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, MinimalPlugins, World},
};
use gdtf_battle_sim::{
    Accuracy, Aiming, BaseSpread, BattleSeed, BraceStairCells, Cell, CellLevel, CombatTuning,
    CoverLedger, DamageProfile, DamageType, Direction, Facing, Faction, FatalBias, FireMode,
    FireModeSpec, Handedness, HandlingProfile, Hp, InflictedWounds, InjuryRng, Kickback, Level,
    LifeState, LootRng, Luck, Magazine, MagazineSize, MarchKind, ModeConeMult, ModeKind, ModeShots,
    ModeTuPercent, OccupancyGrid, OccupancyMaintenancePlugin, PlayerFaction, Position, ProcgenRng,
    ReloadTu, SeverityRng, Shooting, ShotRng, Shove, SimPos, SlabLedger, SquadVisibility, Stable,
    Stance, StanceKind, SurfaceGrid, Toughness, Tu, TuMax, VerticalLinkGraph, WeaponBundle,
    WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WieldedBy, Wounds,
    acts::{FireRequested, SimActsPlugin},
    march_vector,
};

/// The per-round weapon damage the shooter deals to the ground — an arbitrary (NOT
/// shipped-tuning) magnitude. The test asserts the accumulator grows by exactly THIS value
/// per round and sums it over the barrage, never pinning a balance number.
const PER_ROUND_DAMAGE: i32 = 25;

/// The number of single rounds fired at the ground in the barrage — small + bounded (the
/// monotonic-sum proof samples the accumulator after each).
const ROUNDS: u32 = 5;

/// The shooter cell — on the UPPER storey (level 1), so a straight-DOWN shot descends
/// through the open z=0 column and exits the bottom into the ground.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(4, 4), Level::new(1))
}

/// The ground-plane cell the round exits the bottom through — the shooter's `(x, y)` at
/// level 0 (the accumulator keys on the ground-plane `Cell`).
const fn ground_cell() -> Cell {
    Cell::new(4, 4)
}

/// The aim cell — directly BELOW the shooter (same x/y, storey 0), so the central axis
/// tilts straight down through the open column to the ground.
const fn aim_cell() -> Cell {
    Cell::new(4, 4)
}

/// A single-shot fire-mode (one round per shot) — so each `FireRequested` lands exactly one
/// round on the ground, letting the test SUM the per-round accrual (C7(b)).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// Build the real-path app: `MinimalPlugins` + `SimActsPlugin` (the production
/// `dispatch_fire` — the `GroundAccrued` PRODUCER) + `OccupancyMaintenancePlugin` (the
/// production `sync_accrued_ground` — the CONSUMER that accrues onto the surface grid),
/// plus the sim resources `fire()` reads. The `GroundAccrued` buffer is registered by BOTH
/// plugins (idempotent), so the bridge's message reaches the maintenance system.
fn bridge_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    // GTW-14: five per-subsystem RNG streams from the battle seed.
    let seed = BattleSeed::new(0x0660_0366);
    app.insert_resource(ShotRng::from_root(seed));
    app.insert_resource(SeverityRng::from_root(seed));
    app.insert_resource(LootRng::from_root(seed));
    app.insert_resource(InjuryRng::from_root(seed));
    app.insert_resource(gdtf_battle_sim::InjuryTables::default());
    app.insert_resource(gdtf_battle_sim::InjuryRegistry::default());
    app.insert_resource(ProcgenRng::from_root(seed));
    app.insert_resource(CombatTuning::default());
    app.insert_resource(PlayerFaction::new(Faction::new(1)));
    // The cover ledger the shared aim / fire path reads (no cover here — the round strikes
    // the ground, not cover). Seeded empty; asserted UNTOUCHED (C7(c)).
    app.insert_resource(CoverLedger::new());
    // The slab ledger (empty — no slab in the open column). Asserted UNTOUCHED (C7(c)).
    app.insert_resource(SlabLedger::new());
    // GTW-392: `dispatch_fire` reads `Res<BraceStairCells>` — seed an empty set.
    app.insert_resource(BraceStairCells::empty());
    // The other Simulate-band dispatch systems (move / walk) read these.
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    // GTW-396: `dispatch_move` reads `Res<FloorCostGrid>` — seed a uniform grid at the
    // default open cost so the band validates (this test fires at the ground, never moves).
    let default_open = gdtf_battle_sim::tuning::CombatTuning::default()
        .move_costs
        .open;
    app.insert_resource(gdtf_battle_sim::FloorCostGrid::new(default_open, []));
    app
}

/// Spawn an armed, alive, loaded, aiming PLAYER-faction shooter at the shooter cell facing
/// East (the aim point is directly below, so the central axis tilts straight down to the
/// ground — East facing is in-arc enough that the firing-arc gate proceeds). Carries the
/// full shooter-query + target-query component set, with a known per-round damage so the
/// test can sum the accrual.
fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("dirt-kicker".to_owned()),
        // A near-zero cone so the round stays on the central axis (straight down at the ground).
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        // A KNOWN per-round damage — the test asserts the ground accumulator grows by this
        // value per round (the relation, never a shipped magnitude).
        DamageProfile::new(
            WeaponDamage::new(PER_ROUND_DAMAGE),
            WeaponPunch::new(10),
            WeaponShred::new(4),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(30, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode()]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = world
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            // A full TU pool (refilled before each shot — the test takes many single shots
            // with no End-Turn regen between them). `Tu` is a `u8`.
            Tu::new(250),
            // A low TuMax so each single-shot fire costs only a few TU of the pool.
            TuMax::new(10),
            Faction::new(1),
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
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`); the
    // `WieldedBy` insert hook populates the ganger's `Wields` synchronously.
    world.spawn((WieldedBy::new(shooter), bundle));
    shooter
}

/// March a probe ray straight DOWN from the shooter's cell-center through the open column,
/// reading the LIVE grids — returns whether the probe strikes the GROUND (the shared
/// `march_vector`; `MarchKind::Ground` fires on a bottom exit). Confirms the geometry the
/// fired round flies, so the test's accrual is genuinely a ground strike.
fn probe_strikes_ground(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    // A muzzle in the shooter's cell-center on the upper storey (z within level 1), firing
    // straight DOWN through the open column.
    let muzzle = SimPos::new(4.5, 4.5, 1.5);
    let dir = Vec3::new(0.0, 0.0, -1.0);
    let result = march_vector(
        muzzle,
        dir,
        occupancy,
        surface,
        cover,
        tuning,
        shooter_cell(),
        |_| false,
    );
    matches!(result.kind, MarchKind::Ground)
}

/// Fire ONE single round at the ground, then settle the producer→consumer hand-off.
///
/// Refills the shooter's `Tu` to full FIRST (the test takes many single shots with no
/// End-Turn regen between them, so without a refill the shooter would run dry and a later
/// round would fail-close before reaching the ground). A test-body `World` write
/// (`bevy-traps.md` #7 carve-out), not a sim system.
fn fire_one_round(app: &mut App, shooter: Entity) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(),
        aim_cell(),
        Level::new(0),
    ));
    // Buffered messages persist a frame, so a couple of updates settle the
    // dispatch_fire → sync_accrued_ground hand-off regardless of intra-frame order.
    app.update();
    app.update();
}

/// C7(a) + C7(b) + C7(c) — the full ground-accrual bridge through the REAL wiring.
#[test]
fn fired_rounds_accrue_ground_damage_monotonically_without_touching_other_state() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    // Seed an EMPTY surface grid (open column — no slabs to stop the descent) and an empty
    // occupancy grid. The ground accumulator starts at zero for the target cell.
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    let tuning = CombatTuning::default();
    let cover_probe = CoverLedger::new();
    let cell = ground_cell();

    // BEFORE: the column is open and a probe ray fired DOWN strikes the GROUND (so the
    // fired round genuinely lands a ground strike), and the accumulator reads zero.
    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            probe_strikes_ground(occ, surface_res, &cover_probe, &tuning),
            "BEFORE: a probe ray fired DOWN must strike the GROUND (the open column the \
             fired round descends through)",
        );
        assert_eq!(
            *surface_res.ground_damage(&cell),
            0,
            "BEFORE: the target cell's ground accumulator must read zero (no strike yet)",
        );
    }

    // Snapshot the OTHER state BEFORE the barrage — C7(c) compares against these AFTER.
    let hp_before = *app
        .world()
        .get::<Hp>(shooter)
        .copied()
        .unwrap_or(Hp::new(0));
    let wounds_before = *app
        .world()
        .get::<Wounds>(shooter)
        .copied()
        .unwrap_or(Wounds::new(0));

    // Fire a barrage of single rounds, sampling the accumulator after each so we can prove
    // it is MONOTONIC (strictly growing, never decreasing) AND that it sums the per-round
    // weapon damage. A bounded loop (never an infinite-loop hazard).
    let mut last_total = 0_u32;
    for n in 1..=ROUNDS {
        fire_one_round(&mut app, shooter);
        let total = *app.world().resource::<SurfaceGrid>().ground_damage(&cell);
        // C7(b): MONOTONIC — the accumulator strictly grew over the previous read and never
        // decreased.
        assert!(
            total > last_total,
            "C7(b): the ground accumulator must STRICTLY GROW each strike (monotonic) — \
             round {n}: {total} vs previous {last_total}",
        );
        // C7(a) + C7(b): the running total is the SUM of the per-round weapon damage (n
        // strikes of PER_ROUND_DAMAGE), read relative to the weapon the test built — never a
        // pinned shipped magnitude.
        let expected_sum = u32::try_from(PER_ROUND_DAMAGE).unwrap_or(0) * n;
        assert_eq!(
            total, expected_sum,
            "C7(a/b): after {n} ground strikes the accumulator must equal the SUM of the \
             rounds' weapon_damage ({expected_sum})",
        );
        last_total = total;
    }

    // C7(a): the final accumulated total is the full barrage's summed weapon damage on the
    // CORRECT cell — strictly positive and exactly ROUNDS × the per-round damage.
    let final_total = *app.world().resource::<SurfaceGrid>().ground_damage(&cell);
    assert_eq!(
        final_total,
        u32::try_from(PER_ROUND_DAMAGE).unwrap_or(0) * ROUNDS,
        "C7(a): the final ground total must be the summed weapon_damage of the whole barrage",
    );

    // C7(c): NO ganger / cover / slab state was mutated by the ground barrage. The shooter's
    // HP + Wounds are unchanged (a ground strike wounds nobody), the cover + slab ledgers are
    // still EMPTY (no structural HP was spent), and the surface grid holds NO slab state (the
    // accrual touched only the ground accumulator).
    assert_eq!(
        *app.world()
            .get::<Hp>(shooter)
            .copied()
            .unwrap_or(Hp::new(0)),
        hp_before,
        "C7(c): a ground barrage must not change the shooter's HP (it wounds no ganger)",
    );
    assert_eq!(
        *app.world()
            .get::<Wounds>(shooter)
            .copied()
            .unwrap_or(Wounds::new(0)),
        wounds_before,
        "C7(c): a ground barrage must not change any ganger's Wounds",
    );
    assert!(
        app.world()
            .resource::<CoverLedger>()
            .peek(&CellLevel::new(cell, Level::new(0)))
            .is_none(),
        "C7(c): a ground barrage must not insert / deplete the cover ledger",
    );
    assert!(
        app.world()
            .resource::<SlabLedger>()
            .peek(&CellLevel::new(cell, Level::new(0)))
            .is_none(),
        "C7(c): a ground barrage must not insert / deplete the slab ledger",
    );
}
