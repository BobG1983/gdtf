//! GTW-365 C9 — the slab fire→deplete→`SlabDestroyed`→`destroy_slab`→LOS-rebuild bridge
//! drives the REAL downstream wiring.
//!
//! A round that strikes a floor/roof slab and depletes its HP to zero makes the WIRED
//! `dispatch_fire` emit a buffered `SlabDestroyed`, which the WIRED `sync_destroyed_slab`
//! folds into the `SurfaceGrid` via `destroy_slab` — setting the slab
//! `SlabState::Destroyed` so a round AND line of sight pass through the hole. This test
//! drives the ACTUAL `dispatch_fire` + `sync_destroyed_slab` systems (NOT a
//! reimplementation): it builds a real headless app with the production plugins, fires a
//! `FireRequested` straight UP through a low-HP slab, and asserts:
//!
//! - **C9(a)** the slab depletes over MULTIPLE persistent hits, THEN destroys (the
//!   `SurfaceGrid` flips to `SlabState::Destroyed` only after enough strikes);
//! - **C9(b)** a destroyed slab stops blocking rounds AND LOS passes through it — proved
//!   via the REAL shared `march_vector` (BEFORE: a probe ray STOPS on the slab; AFTER: it
//!   climbs THROUGH to Miss), the same march `has_los` uses;
//! - **C9(c)** a destroyed slab is NOT walkable — the `VerticalLinkGraph` is unchanged by
//!   the destruction (no vertical-link / pathfinding change; movement is out of scope).
//!
//! Render-free, zero pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md`
//! #7 carve-out) — no helper here takes `&mut World` / `&World`.

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    armor::{ArmorHardness, ArmorProtection},
    cover::CoverLedger,
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    march::{MarchKind, march_vector},
    occupancy_sync::OccupancyMaintenancePlugin,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position, SimPos,
        Stance, StanceKind, Tu,
    },
    slab::{BraceStairCells, SlabEntry, SlabHp},
    surface::{SlabState, SurfaceGrid},
    test_support::{SimAppBuilder, empty_slab_ledger, single_mode},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, Handedness,
        HandlingProfile, Kickback, MagazineSize, Shove, Stable, WeaponBundle, WeaponDamage,
        WeaponName, WeaponPunch, WeaponShred, WieldedBy,
    },
};

/// The shooter cell — on the GROUND storey, directly BELOW the slab cell, so its shot
/// climbs straight up through the z-boundary the slab spans.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(0))
}

/// The slab key — keyed at the UPPER level of the boundary (the floor of storey 1 / the
/// roof of storey 0), directly above the shooter's `(x, y)`.
fn slab_key() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(1))
}

/// The aim cell — the cell directly ABOVE the shooter (same x/y, storey 1), so the
/// central axis points straight up through the slab.
const fn aim_cell() -> Cell {
    Cell::new(5, 5)
}

/// Build the real-path app: `MinimalPlugins` + `SimActsPlugin` (the production
/// `dispatch_fire` — the `SlabDestroyed` PRODUCER) + `OccupancyMaintenancePlugin` (the
/// production `sync_destroyed_slab` — the CONSUMER that sets the surface grid's slab
/// Destroyed), plus the sim resources `fire()` reads. The `SlabDestroyed` buffer is
/// registered by BOTH plugins (idempotent), so the bridge's message reaches the
/// maintenance system.
fn bridge_app() -> App {
    // The canonical `with_acts` litany (GTW-576) seeds the grids + RNG streams + tuning the
    // whole Simulate band validates against; only the seed + the non-player PlayerFaction
    // differ from the defaults here.
    let mut app = SimAppBuilder::new()
        .with_seed(0x51AB_C0DE)
        .with_acts()
        .with_player_faction(1)
        .build();
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Spawn an armed, alive, loaded, aiming PLAYER-faction shooter at the shooter cell
/// facing UP-ish (East facing is in-arc enough; the aim point is directly overhead, so
/// the central axis tilts straight up to the slab). Carries the full shooter-query +
/// target-query component set. Uses LOW per-round damage so the slab survives the first
/// hits and depletes over SEVERAL strikes (C9(a)).
fn spawn_shooter(world: &mut World) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("chipper".to_owned()),
        // A near-zero cone so the round stays on the central axis (straight up at the slab).
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        // LOW per-round damage so NO single round destroys the slab — it chips away over
        // multiple persistent hits (C9(a)). Punch beats the slab's low hardness.
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(30),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(30),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
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
            // A full TU pool (refilled before each shot in `fire_one_round`, since the test
            // takes many single shots without an End-Turn regen). `Tu` is a `u8`.
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

/// A low-HP, low-armor slab entry pre-inserted at the slab key — small HP so a HANDFUL
/// of the low-damage rounds (not one) deplete it, and low armor so the punch breaches.
/// The fold uses this inserted entry (lazy-seed only fills an ABSENT key); the magnitude
/// is chosen only to keep the slab in the multi-hit regime, never as a balance claim.
const fn low_hp_slab() -> SlabEntry {
    SlabEntry::seeded(
        SlabHp::new(120),
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

/// March a probe ray straight UP from just under the slab through the slab cell, reading
/// the LIVE grids — returns whether the probe STOPS on the slab (the shared
/// `march_vector` `has_los` uses; `MarchKind::Slab` ONLY fires on an intact slab).
fn probe_stops_on_slab(
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> bool {
    // A muzzle in the shooter's cell-center just below the z-boundary, firing straight UP.
    let muzzle = SimPos::new(5.5, 5.5, 0.5);
    let dir = bevy::math::Vec3::new(0.0, 0.0, 1.0);
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
    matches!(result.kind, MarchKind::Slab)
}

/// Fire ONE single round at the slab cell, then settle the producer→consumer hand-off.
///
/// Refills the shooter's `Tu` to full FIRST (the test takes many single shots with no
/// End-Turn regen between them, so without a refill the shooter would run dry and a later
/// round would fail-close before reaching the slab). A test-body `World` write
/// (`bevy-traps.md` #7 carve-out), not a sim system.
fn fire_one_round(app: &mut App, shooter: Entity) {
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(shooter) {
        *tu = Tu::new(250);
    }
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        aim_cell(),
        Level::new(1),
    ));
    // Buffered messages persist a frame, so a couple of updates settle the
    // dispatch_fire → sync_destroyed_slab hand-off regardless of intra-frame order.
    app.update();
    app.update();
}

/// C9(a) + C9(b) + C9(c) — the full slab-destruction bridge through the REAL wiring.
#[test]
fn fired_rounds_deplete_then_destroy_slab_and_open_los_without_walkability() {
    let mut app = bridge_app();
    let shooter = spawn_shooter(app.world_mut());

    // Seed the surface grid with an INTACT slab at the boundary above the shooter, and a
    // pristine vertical-link graph (the C9(c) baseline). Pre-insert the low-HP slab entry
    // in the model ledger at the same key.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key(), SlabState::Present);
    app.insert_resource(surface);
    let mut slab = empty_slab_ledger();
    slab.insert(slab_key(), low_hp_slab());
    app.insert_resource(slab);
    // GTW-392: `dispatch_fire` reads `Res<BraceStairCells>` — seed an empty set (no
    // stair links in this geometry — shooter fires vertically through the slab).
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(OccupancyGrid::new());
    // The vertical-link count BEFORE destruction — C9(c) compares against this AFTER
    // (the graph is not `PartialEq`, so the count + the slab-key departure set are the
    // observable "unchanged" witnesses).
    let links_before = app.world().resource::<VerticalLinkGraph>().links().count();
    assert!(
        app.world()
            .resource::<VerticalLinkGraph>()
            .links_from(&slab_key())
            .next()
            .is_none(),
        "BEFORE: the intact slab key has no departing vertical link (the C9(c) baseline)",
    );

    let tuning = CombatTuning::default();
    let cover_probe = CoverLedger::new();

    // BEFORE: the slab is intact, blocks the boundary, and a probe ray STOPS on it.
    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        assert_eq!(
            surface_res.slab_state(&slab_key()),
            SlabState::Present,
            "BEFORE: the slab must be intact (Present)",
        );
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            probe_stops_on_slab(occ, surface_res, &cover_probe, &tuning),
            "BEFORE: a probe ray fired UP must STOP on the intact slab (it blocks rounds + LOS)",
        );
    }

    // C9(a): fire ONE round — the low-damage round must DAMAGE the slab, not destroy it
    // (the persistent pool is still above zero), so the slab stays Present.
    fire_one_round(&mut app, shooter);
    assert_eq!(
        app.world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key()),
        SlabState::Present,
        "C9(a): one low-damage round must only DAMAGE the slab — it stays Present (the pool \
         persists above zero)",
    );

    // Keep firing single rounds until the PERSISTENT pool drains to zero and the bridge
    // flips the slab Destroyed. A bounded loop (never an infinite-loop hazard) — the pool
    // is finite and each round chips it, so destruction must occur within the bound; the
    // assertion after the loop fails loudly if it somehow did not.
    let mut strikes_to_destroy = 1_u32; // the first round above already landed
    for _ in 0..64 {
        if app
            .world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key())
            == SlabState::Destroyed
        {
            break;
        }
        fire_one_round(&mut app, shooter);
        strikes_to_destroy += 1;
    }

    // C9(a): destruction required MORE THAN ONE strike (a persistent multi-hit depletion),
    // and the bridge ultimately set the surface grid's slab Destroyed.
    assert!(
        strikes_to_destroy > 1,
        "C9(a): the slab must deplete over MULTIPLE persistent strikes, not one — took \
         {strikes_to_destroy}",
    );
    assert_eq!(
        app.world()
            .resource::<SurfaceGrid>()
            .slab_state(&slab_key()),
        SlabState::Destroyed,
        "C9(a): the bridge (dispatch_fire → SlabDestroyed → sync_destroyed_slab → \
         destroy_slab) must set the slab Destroyed once its persistent pool hits zero",
    );

    // C9(b): AFTER destruction — a round AND line of sight pass through the hole. The
    // shared march flies THROUGH a Destroyed slab (it stops only on Present), so the
    // probe no longer reports MarchKind::Slab.
    {
        let surface_res = app.world().resource::<SurfaceGrid>();
        let occ = app.world().resource::<OccupancyGrid>();
        assert!(
            !probe_stops_on_slab(occ, surface_res, &cover_probe, &tuning),
            "C9(b): a probe ray fired UP must now PASS THROUGH the destroyed slab (rounds + LOS \
             pass — the shared march honors SlabState::Destroyed)",
        );
    }

    // C9(c): the destroyed slab is NOT walkable — the vertical-link graph is UNCHANGED by
    // the destruction (no vertical-link / pathfinding mutation; movement is out of scope).
    let links_after = app.world().resource::<VerticalLinkGraph>().links().count();
    assert_eq!(
        links_before, links_after,
        "C9(c): destroying a slab must NOT change the vertical-link graph's link count (a \
         destroyed slab is NOT walkable — no pathfinding / vertical-link change)",
    );
    // And the slab key never became a traversable vertical link (no departing link added).
    assert!(
        app.world()
            .resource::<VerticalLinkGraph>()
            .links_from(&slab_key())
            .next()
            .is_none(),
        "C9(c): a destroyed slab must add NO vertical link from its key (not walkable)",
    );
}
