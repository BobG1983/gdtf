//! GTW-364 C8(c) — the fire→deplete→message BRIDGE drives the REAL downstream wiring.
//!
//! A round that strikes a piece of cover and depletes its HP to zero makes the WIRED
//! `dispatch_fire` emit a buffered `CoverDestroyed`, which the WIRED `sync_destroyed_cover`
//! folds into the occupancy grid's append-only destroyed-cover set — freeing the cell so it
//! stops blocking and a previously-blocked LOS/path opens. This test drives the ACTUAL
//! `dispatch_fire` + `sync_destroyed_cover` systems (NOT a reimplementation): it builds a real
//! headless app with the production plugins, fires a `FireRequested` at a low-HP cover cell,
//! and asserts the smashed cell unblocks via the real maintenance system.
//!
//! Render-free, zero pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md` #7
//! carve-out) — no helper here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, ReloadTu},
    march::{MarchKind, march_vector},
    occupancy::TerrainKind,
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, SimPos,
        Stance, StanceKind, Tu,
    },
    surface::SurfaceGrid,
    test_support::{SimAppBuilder, single_mode},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

/// The shooter cell — West of the cover, on the ground storey.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), Level::new(0))
}

/// The cover cell — directly East of the shooter, straight ahead of an East facing (in
/// the firing arc), so the central axis aims at the cover's own band.
fn cover_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), Level::new(0))
}

/// Build the real-path app: `MinimalPlugins` + `SimActsPlugin` (the production
/// `dispatch_fire` — the message PRODUCER) + `OccupancyMaintenancePlugin` (the production
/// `sync_destroyed_cover` — the message CONSUMER that frees the cell), plus the sim
/// resources `fire()` reads. The `CoverDestroyed` buffer is registered by BOTH plugins
/// (idempotent), so the bridge's message reaches the maintenance system.
fn bridge_app() -> App {
    // The canonical `with_acts` litany (GTW-576) seeds the grids + RNG streams + tuning the
    // whole Simulate band validates against; only the seed + the non-player PlayerFaction
    // differ from the defaults here.
    let mut app = SimAppBuilder::new()
        .with_seed(0xC0BA_17C0)
        .with_acts()
        .with_player_faction(1)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Spawn an armed, alive, loaded, aiming PLAYER-faction shooter at the shooter cell facing
/// East at the cover, with a HIGH-damage weapon + tight cone so a single round lands on the
/// cover and breaches its low HP. Carries the full shooter-query + target-query component
/// set (its own liveness reads through the target query).
fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("breacher".to_owned()),
        // A near-zero cone so the round stays on the central axis (onto the cover band).
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        // High damage + penetration so the single round breaches the low-HP cover.
        DamageProfile::new(
            WeaponDamage::new(200),
            WeaponPunch::new(80),
            WeaponShred::new(40),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode(0.2, 1)]),
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
            Tu::new(250),
            TuMax::new(100),
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

/// A low-HP, HIGH-band cover entry — small HP + low armor so a single high-damage round
/// destroys it. HIGH band so a standing shooter's aim lands squarely on it.
const fn low_hp_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

/// March a probe ray straight East from the shooter into the cover cell, reading the LIVE
/// grids — returns whether the probe STOPS on cover at the cover cell.
fn probe_stops_on_cover(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    // A muzzle at the shooter cell-center, firing due East along the cover band's storey.
    let muzzle = SimPos::new(2.5, 5.5, 0.9);
    let dir = bevy::math::Vec3::new(1.0, 0.0, 0.0);
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
    matches!(result.kind, MarchKind::Cover(_))
}

/// C8(c) — firing a destroying round at a cover cell, through the REAL `dispatch_fire`
/// bridge + `sync_destroyed_cover` wiring, frees the cell: it stops blocking AND a
/// previously-blocked LOS/path now passes. Asserts the MECHANISM (the exclusion set +
/// the unblocked march), never a magnitude.
#[test]
fn fired_round_destroys_cover_and_the_bridge_frees_the_cell() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    // Seed the cover: a BLOCKING terrain marker in the occupancy grid AND a low-HP entry in
    // the model ledger at the same cell (the authored cover the round will smash).
    let mut occupancy = OccupancyGrid::new();
    occupancy.set_terrain(cover_cell(), TerrainKind::Cover);
    app.insert_resource(occupancy);
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell(), low_hp_cover());
    app.insert_resource(cover);

    // BEFORE: the cover blocks, and a probe LOS/path stops on it. Read snapshots off the
    // world (the test body's read of the live resources).
    let tuning = CombatTuning::default();
    let surface_before = SurfaceGrid::new();
    {
        let grid = app.world().resource::<OccupancyGrid>();
        let cover_res = app.world().resource::<CoverLedger>();
        assert!(
            grid.is_blocked(&cover_cell()),
            "BEFORE: the standing cover cell must block",
        );
        assert!(
            !grid.is_cover_destroyed(&cover_cell()),
            "BEFORE: the cover cell must not yet be in the destroyed-cover set",
        );
        assert!(
            probe_stops_on_cover(grid, &surface_before, cover_res, &tuning),
            "BEFORE: a probe LOS/path must STOP on the standing cover",
        );
    }

    // Fire at the cover cell. dispatch_fire resolves the shot (depleting the cover HP and
    // emitting CoverDestroyed via the bridge); sync_destroyed_cover then folds the message
    // into the grid's destroyed-cover set. Buffered messages persist a frame, so a couple
    // of updates settle the producer→consumer hand-off regardless of intra-frame order.
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(cover_cell().x, cover_cell().y),
        Level::new(0),
    ));
    app.update();
    app.update();

    // AFTER: the bridge drove the REAL maintenance — the cell is in the append-only
    // destroyed-cover set (mark_cover_destroyed), so it no longer blocks AND a probe
    // LOS/path now passes through it.
    let grid = app.world().resource::<OccupancyGrid>();
    let cover_res = app.world().resource::<CoverLedger>();
    assert!(
        grid.is_cover_destroyed(&cover_cell()),
        "AFTER: sync_destroyed_cover must have marked the smashed cell destroyed (the bridge \
         drove the real wiring) — got destroyed-set miss",
    );
    assert!(
        !grid.is_blocked(&cover_cell()),
        "AFTER: a destroyed cover cell must stop blocking (the freed cell)",
    );
    assert!(
        !probe_stops_on_cover(grid, &surface_before, cover_res, &tuning),
        "AFTER: a probe LOS/path must now PASS through the freed cell (a previously-blocked \
         sightline opened)",
    );
}
