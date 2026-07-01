//! GTW-523 — the fall mechanic end-to-end through the REAL wiring (`apply_falls`).
//!
//! A slab destroyed under a standing ganger makes the WIRED `apply_falls` (the production
//! system, NOT a reimplementation) drop the faller, apply weight-free fall damage + injury
//! through the shared resolve/apply + injury pipeline, and emit `FallOccurred`. This test
//! builds a real headless app with the production plugins (`SimActsPlugin` +
//! `OccupancyMaintenancePlugin` + `FallsPlugin`), writes a buffered `SlabDestroyed`, drives
//! `app.update()`, and asserts the whole clause contract:
//!
//! - **QA(1)** a 3-storey column, ganger on level 2, `SlabDestroyed` at (cell,2) → `Position`
//!   drops to the highest Present/ground + storeys == expected; the faller keys off
//!   `level == 2`, NOT level+1 (an EXPLICIT roof-decoy ganger at level 3 does NOT fall);
//! - **QA(2)** a multi-storey drop through an `Absent` intermediate → the first Present/ground;
//! - **QA(3)** a ganger on a DIFFERENT level does not move;
//! - **QA(4)** a stair lower-endpoint occupant is BRACED — it does not fall;
//! - **QA(5)** Hp dropped + a seeded injury + `InjuryInflicted` fired + damage monotone in
//!   storeys;
//! - **QA(6)** determinism — the same seed twice yields identical outcomes;
//! - **QA(7)** a hot-edit of `per_storey_damage` → different damage (the FORMULA, not a
//!   shipped magnitude);
//! - **QA(8)** regression — a destroyed slab is NON-pathable (the `VerticalLinkGraph` is
//!   untouched) and LOS flies THROUGH the hole (the shared `march_vector`).
//!
//! Render-free, zero pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md`
//! #7 carve-out) — no helper here takes `&mut World` / `&World`.

use bevy::{
    app::{App, Update},
    math::Vec3,
    prelude::{Entity, MessageReader, MinimalPlugins, ResMut, Resource, World},
};
use gdtf_battle_sim::{
    BattleSeed, Cell, CellLevel, CombatTuning, CoverLedger, Faction, FallOccurred, Hp,
    InflictedWounds, InjuryRng, Level, LifeState, LootRng, Luck, MarchKind, OccupancyGrid,
    OccupancyMaintenancePlugin, PerStoreyDamage, PlayerFaction, Position, ProcgenRng, SeverityRng,
    ShotRng, SimPos, SlabState, SquadVisibility, Stance, StanceKind, SurfaceGrid, Toughness, Tu,
    TuMax, VerticalLinkGraph, Wounds, acts::InjuryInflicted, falls::FallsPlugin, march_vector,
    occupancy_sync::SlabDestroyed,
};

/// The SHIPPED combat tuning parsed from the real `assets/core_tuning/combat.tuning.ron` (the
/// realistic severity edges / scaling — NOT the low `CombatTuning::default()` placeholder
/// ladder). Falls back to the default on a parse failure (defensive; the tuning-test suite
/// already guards the parse-OK path).
fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../assets/core_tuning/combat.tuning.ron");
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

/// The shared column x/y every faller stands in (the cell the destroyed slab keys).
const COL_X: i32 = 5;
const COL_Y: i32 = 5;

/// The `(cell, level)` key of the destroyed slab / the standing faller's storey.
const fn column_cell() -> Cell {
    Cell::new(COL_X, COL_Y)
}

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
const SEED: u64 = 0x0523_FA11_DEAD_BEEF;

// ── Test-local signal recorders ───────────────────────────────────────────────
//
// A `MessageReader` sees only the current+previous update, so recording every signal into a
// resource the moment it is emitted lets the asserts read the full run history (the gtw507
// `MeleeLog` precedent).

/// Every `FallOccurred` observed across the run.
#[derive(Resource, Default)]
struct FallLog {
    /// One entry per `FallOccurred` emitted.
    falls: Vec<FallOccurred>,
}

/// Every `InjuryInflicted` target observed across the run.
#[derive(Resource, Default)]
struct InjuryLog {
    /// The entity of each `InjuryInflicted` emitted.
    targets: Vec<Entity>,
}

/// Drain `FallOccurred` into the recorder.
fn record_falls(mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>) {
    for signal in reader.read() {
        log.falls.push(*signal);
    }
}

/// Drain `InjuryInflicted` into the recorder (targets only — the applier drains the buffer
/// too, but each `MessageReader` has its own cursor, so both see every message).
fn record_injuries(mut reader: MessageReader<InjuryInflicted>, mut log: ResMut<InjuryLog>) {
    for message in reader.read() {
        log.targets.push(message.target);
    }
}

/// Build the real-path app: `MinimalPlugins` + `SimActsPlugin` (the `dispatch_fire`
/// `apply_falls` orders `.after`) + `OccupancyMaintenancePlugin` (the `sync_destroyed_slab`
/// it orders `.after`) + the GTW-523 `FallsPlugin` (the `FallOccurred` buffer +
/// `apply_falls`). Inserts the sim resources the falls fold + the sibling dispatch systems
/// read. The `Simulate` band here has NO `BattleInProgress` gate (that gate lives in
/// `BattleSimPlugin`), so `apply_falls` runs unconditionally over the inserted resources —
/// the focused-harness pattern the fire/slab bridge tests use.
fn falls_app(seed: u64, per_storey: PerStoreyDamage) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(gdtf_battle_sim::acts::SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin);
    let root = BattleSeed::new(seed);
    // The two streams the fall fold draws (SeverityRng + InjuryRng) + the others the sibling
    // Simulate-band systems read (ShotRng / LootRng / ProcgenRng).
    app.insert_resource(ShotRng::from_root(root));
    app.insert_resource(SeverityRng::from_root(root));
    app.insert_resource(InjuryRng::from_root(root));
    app.insert_resource(LootRng::from_root(root));
    app.insert_resource(ProcgenRng::from_root(root));
    app.insert_resource(gdtf_battle_sim::InjuryTables::default());
    app.insert_resource(gdtf_battle_sim::InjuryRegistry::default());
    // The SHIPPED combat tuning (the real severity edges / scaling — the `CombatTuning::default()`
    // is a low placeholder ladder that turns even a shallow fall Fatal), with the per-storey
    // magnitude under test overlaid (QA(7) varies it). Parsed from the same real file the app
    // loads (the tuning-test `include_str!` precedent) so the fall's severity bucketing is
    // realistic; a parse failure falls back to the default (the test still runs).
    let mut tuning = shipped_tuning();
    tuning.per_storey_damage = per_storey;
    app.insert_resource(tuning);
    app.insert_resource(PlayerFaction::new(Faction::new(1)));
    app.insert_resource(CoverLedger::new());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    // The other Simulate-band dispatch systems (dispatch_fire / dispatch_move) read these —
    // the band runs unconditionally here (no BattleInProgress gate outside BattleSimPlugin),
    // so every member's Res must exist even though this test only exercises apply_falls.
    app.insert_resource(gdtf_battle_sim::SlabLedger::new());
    app.insert_resource(gdtf_battle_sim::BraceStairCells::empty());
    let default_open = CombatTuning::default().move_costs.open;
    app.insert_resource(gdtf_battle_sim::FloorCostGrid::new(default_open, []));
    // The signal recorders — added after the plugins so the FallOccurred / InjuryInflicted
    // buffers exist, so the run's full history is queryable after the app.update()s.
    app.init_resource::<FallLog>();
    app.init_resource::<InjuryLog>();
    app.add_systems(Update, (record_falls, record_injuries));
    app
}

/// Spawn a standing, alive faller with the falls-query + fold component set at `(cell,
/// level)`, with a large HP/Wounds pool (so a fall wounds but rarely kills — a clean HP
/// measurement). Bare flesh (no worn armor). Returns its entity.
fn spawn_faller(world: &mut World, level: u8) -> Entity {
    world
        .spawn((
            Position::new(CellLevel::new(column_cell(), Level::new(level))),
            Stance::new(StanceKind::Standing),
            Faction::new(1),
            Tu::new(100),
            TuMax::new(100),
            Hp::new(1000),
            Wounds::new(200),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id()
}

/// Read a faller's current `Position` level (its storey index).
fn level_of(app: &App, entity: Entity) -> u8 {
    let pos = app
        .world()
        .get::<Position>(entity)
        .copied()
        .unwrap_or_else(Position::default);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is a storey index in 0..MAX_LEVELS, so the i32 -> u8 narrowing cannot wrap"
    )]
    let z = pos.z as u8;
    z
}

/// Read a faller's current `Hp`.
fn hp_of(app: &App, entity: Entity) -> u16 {
    *app.world().get::<Hp>(entity).copied().unwrap_or(Hp::new(0))
}

/// Write a `SlabDestroyed` at the column cell + `level`, then drive two updates so the
/// buffered signal reaches `apply_falls` regardless of intra-frame order (buffered messages
/// persist a frame). A test-body `World` write (`bevy-traps.md` #7 carve-out).
fn destroy_slab_and_settle(app: &mut App, level: u8) {
    app.world_mut()
        .write_message(SlabDestroyed::new(CellLevel::new(
            column_cell(),
            Level::new(level),
        )));
    app.update();
    app.update();
}

/// The full run history of `FallOccurred` signals (from the test-local recorder).
fn fall_signals(app: &App) -> Vec<FallOccurred> {
    app.world()
        .get_resource::<FallLog>()
        .map(|log| log.falls.clone())
        .unwrap_or_default()
}

/// Whether ANY `InjuryInflicted` addressed `entity` across the run (from the recorder).
fn injury_fired_for(app: &App, entity: Entity) -> bool {
    app.world()
        .get_resource::<InjuryLog>()
        .is_some_and(|log| log.targets.contains(&entity))
}

// ── QA(1) + QA(3): faller predicate keys off level == N, NOT N+1 ──────────────

/// QA(1): a 3-storey column, faller on level 2, `SlabDestroyed` at (cell, 2) → the faller's
/// `Position` drops to the ground (highest support below) and the storeys distance is 2.
/// QA(3): an EXPLICIT roof-decoy faller at level 3 (== `destroyed_level` + 1, the ROOF — the
/// WRONG actor) does NOT move, proving the predicate keys off `level == N`, not `N + 1`.
#[test]
fn faller_on_destroyed_slab_level_falls_roof_occupant_does_not() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    // The faller STANDING on the floor of storey 2 (Position.level == 2).
    let faller = spawn_faller(app.world_mut(), 2);
    // The decoy on the ROOF (level 3 == destroyed_level + 1) — the wrong actor; must not fall.
    let roof_decoy = spawn_faller(app.world_mut(), 3);
    // An empty surface grid (no intermediate slabs) → the faller drops to the ground (0).
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2);

    // QA(1): the level-2 faller dropped to the ground (0), a 2-storey fall.
    assert_eq!(
        level_of(&app, faller),
        0,
        "the level-2 faller drops to the ground"
    );
    // QA(3): the roof-decoy at level 3 (destroyed_level + 1) did NOT move — the predicate
    // keys off level == N, not N + 1.
    assert_eq!(
        level_of(&app, roof_decoy),
        3,
        "the roof occupant (level+1) is the WRONG actor and must NOT fall"
    );
    // QA(1): exactly ONE FallOccurred, for the level-2 faller, storeys == 2, from 2 → 0.
    let signals = fall_signals(&app);
    assert_eq!(
        signals.len(),
        1,
        "exactly one fall (the roof decoy did not fall)"
    );
    let signal = signals[0];
    assert_eq!(signal.ganger, faller);
    assert_eq!(signal.from_level, Level::new(2));
    assert_eq!(signal.to_level, Level::new(0));
    assert_eq!(*signal.storeys, 2, "start 2 → land 0 is a 2-storey fall");
}

// ── QA(2): multi-storey drop through an Absent intermediate ───────────────────

/// QA(2): a faller at level 4 over an `Absent` intermediate (level 2, open air) and an
/// intact `Present` floor at level 1 falls THROUGH the open air and lands on the level-1
/// slab (not the ground) — a 3-storey drop.
#[test]
fn multi_storey_drop_through_absent_lands_on_first_present() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(6));
    let faller = spawn_faller(app.world_mut(), 4);
    let mut surface = SurfaceGrid::new();
    // level 1: an intact Present floor (the landing). levels 2/3: Absent (open air).
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 4);

    assert_eq!(
        level_of(&app, faller),
        1,
        "the faller falls through the Absent intermediate and lands on the Present level-1 slab"
    );
    let signals = fall_signals(&app);
    assert_eq!(signals.len(), 1);
    assert_eq!(
        *signals[0].storeys, 3,
        "start 4 → land 1 is a 3-storey fall"
    );
}

// ── QA(3): a ganger on a different level does not move ─────────────────────────

/// QA(3): a faller on a DIFFERENT level than the destroyed slab does not move and no fall
/// fires. (The roof-decoy case above covers level+1 specifically; this covers a level BELOW
/// the destroyed slab.)
#[test]
fn ganger_on_different_level_does_not_fall() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    // A faller on level 1 — BELOW the level-3 destroyed slab, a different storey.
    let elsewhere = spawn_faller(app.world_mut(), 1);
    // Intact floor at level 1 so `elsewhere` is on solid ground (not itself falling).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 3);

    assert_eq!(
        level_of(&app, elsewhere),
        1,
        "a ganger on a different level does not move"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "no fall fires for a ganger off the destroyed level"
    );
}

// ── QA(4): a stair lower-endpoint occupant is braced ──────────────────────────

/// QA(4): a faller standing on an authored STAIR tile at the destroyed slab's level is
/// BRACED — the stair supports it, so it does NOT fall through its own stair (no drop, no
/// damage, no signal).
#[test]
fn stair_lower_endpoint_occupant_is_braced() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let braced = spawn_faller(app.world_mut(), 2);
    app.insert_resource(SurfaceGrid::new());
    // Mark the faller's cell an authored stair tile — the C3 brace.
    let mut occupancy = OccupancyGrid::new();
    occupancy.mark_stair_cell(CellLevel::new(column_cell(), Level::new(2)));
    app.insert_resource(occupancy);
    let hp_before = hp_of(&app, braced);

    destroy_slab_and_settle(&mut app, 2);

    assert_eq!(
        level_of(&app, braced),
        2,
        "a braced stair occupant does not fall"
    );
    assert_eq!(
        hp_of(&app, braced),
        hp_before,
        "a braced occupant takes no fall damage"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "a braced occupant fires no FallOccurred"
    );
}

// ── QA(5): Hp dropped + seeded injury + InjuryInflicted + damage monotone ──────

/// QA(5): a fall drops Hp, fires a seeded `InjuryInflicted` for the faller, and the damage
/// is monotone in storeys (a deeper fall hurts at least as much). A CONTENT-FULL injury
/// table is used so a non-graze wound rolls a real named injury (the `InjuryInflicted` proof).
#[test]
fn fall_drops_hp_fires_injury_and_damage_is_monotone_in_storeys() {
    // A per-storey magnitude tuned so the fall's Torso penetrating damage (per_storey ×
    // storeys) plus the +5 torso part-mod plus the random roll (≤ 10) lands SAFELY in the
    // non-graze, non-fatal band (score ≥ e0=5, < e3=40) for a 2–3 storey fall — so a named
    // injury reliably rolls (Fatal / graze roll NO injury; the tabling rule). Value-agnostic:
    // the RELATION (HP drops, injury fires, monotone) is asserted, never a pinned magnitude.
    let per_storey = PerStoreyDamage::new(6);
    let run = |start_level: u8| -> (u16, bool) {
        let mut app = falls_app(SEED, per_storey);
        // A populated Torso injury table so a Minor/Major/Critical torso wound rolls a named
        // injury (the InjuryInflicted proof); reuses the SHARED (category, severity) pool.
        install_torso_injury_content(&mut app);
        let faller = spawn_faller(app.world_mut(), start_level);
        app.insert_resource(SurfaceGrid::new());
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, start_level);
        let after = hp_of(&app, faller);
        (before - after, injury_fired_for(&app, faller))
    };

    let (lost_2, injury_2) = run(2);
    let (lost_3, injury_3) = run(3);

    assert!(lost_2 > 0, "a fall must drop Hp");
    assert!(
        lost_3 >= lost_2,
        "a deeper fall (3 storeys) must hurt at least as much as a shallow one (2) \
         (2:{lost_2} 3:{lost_3})"
    );
    assert!(
        lost_3 > lost_2,
        "the LINEAR per-storey scaling makes a 3-storey fall hurt strictly more"
    );
    // A seeded non-graze / non-fatal fall wound rolls a named injury from the populated table
    // → an InjuryInflicted message addressed to the faller (the C5 real-wiring proof). Both
    // the 2- and 3-storey falls land in the non-fatal band, so both fire.
    assert!(
        injury_2 && injury_3,
        "a seeded fall wound must roll a named injury and fire InjuryInflicted (2:{injury_2} 3:{injury_3})"
    );
}

// ── QA(6): determinism — same seed twice → identical outcome ──────────────────

/// QA(6): the same seed + same event order yields the IDENTICAL fall outcome (landing +
/// HP loss + storeys) across two independent apps.
#[test]
fn falls_are_deterministic_under_same_seed() {
    let run = || -> (u8, u16, u8) {
        let mut app = falls_app(SEED, PerStoreyDamage::new(20));
        install_torso_injury_content(&mut app);
        let faller = spawn_faller(app.world_mut(), 3);
        let mut surface = SurfaceGrid::new();
        surface.set_slab(
            CellLevel::new(column_cell(), Level::new(1)),
            SlabState::Present,
        );
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, 3);
        let signals = fall_signals(&app);
        let storeys = signals.first().map_or(0, |s| *s.storeys);
        (
            level_of(&app, faller),
            before - hp_of(&app, faller),
            storeys,
        )
    };
    assert_eq!(
        run(),
        run(),
        "same seed + same event order must yield identical fall outcomes"
    );
}

// ── QA(7): hot-edit per_storey_damage → different damage (formula) ────────────

/// QA(7): a hot-edit of `per_storey_damage` changes the blow — a larger leaf deals MORE Hp
/// loss for the same fall. The FORMULA relation is asserted, NOT a shipped magnitude.
#[test]
fn hot_edit_per_storey_damage_changes_fall_damage() {
    let lost = |per_storey: i32| -> u16 {
        let mut app = falls_app(SEED, PerStoreyDamage::new(per_storey));
        let faller = spawn_faller(app.world_mut(), 3);
        let mut surface = SurfaceGrid::new();
        surface.set_slab(
            CellLevel::new(column_cell(), Level::new(1)),
            SlabState::Present,
        );
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, 3);
        before - hp_of(&app, faller)
    };
    let small = lost(5);
    let large = lost(50);
    assert!(
        large > small,
        "a larger per_storey_damage leaf deals more fall damage (small:{small} large:{large})"
    );
}

// ── QA(8): regression — a hole is non-pathable + LOS flies through ────────────

/// QA(8): destroying the slab under a faller does NOT make the hole walkable — the
/// `VerticalLinkGraph` is untouched (no vertical link added) — AND LOS flies THROUGH the
/// hole (the shared `march_vector`: BEFORE a probe STOPS on the intact slab; AFTER it climbs
/// THROUGH to a non-Slab result). The pathing + LOS behavior for a destroyed slab is exactly
/// the GTW-365 contract — GTW-523 adds the fall WITHOUT changing either.
#[test]
fn destroyed_slab_stays_non_pathable_and_los_flies_through() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let _faller = spawn_faller(app.world_mut(), 1);
    // An intact slab at the boundary above the ground cell (level 1 = floor of storey 1).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    // BEFORE: a probe ray straight UP through the boundary STOPS on the intact slab.
    assert!(
        probe_stops_on_slab(&app),
        "an intact slab must STOP a probe ray at the z-boundary (the baseline)"
    );
    let links_before = app.world().resource::<VerticalLinkGraph>().links().count();

    // Destroy the slab (the same signal apply_falls consumed) — a faller at level 1 is on the
    // ground floor (level 0 supports below it), so this exercises the destruction path.
    destroy_slab_and_settle(&mut app, 1);

    // AFTER: the destroyed slab is transparent to the march — the probe climbs THROUGH the
    // hole to a non-Slab result (LOS flies through — UNTOUCHED by GTW-523).
    assert!(
        !probe_stops_on_slab(&app),
        "a destroyed slab must let the probe ray fly THROUGH (LOS passthrough — unchanged)"
    );
    // The VerticalLinkGraph is UNTOUCHED — a destroyed slab adds no vertical link, so the
    // hole stays non-pathable (movement is unchanged — GTW-523 adds no pathing).
    let links_after = app.world().resource::<VerticalLinkGraph>().links().count();
    assert_eq!(
        links_before, links_after,
        "a destroyed slab adds NO vertical link — the hole stays non-pathable (unchanged)"
    );
}

/// March a probe ray straight UP from just under the boundary through the column cell,
/// reading the LIVE grids — returns whether it STOPS on an intact slab (the shared
/// `march_vector` `has_los` uses; `MarchKind::Slab` fires ONLY on an intact slab).
fn probe_stops_on_slab(app: &App) -> bool {
    let occupancy = app.world().resource::<OccupancyGrid>();
    let surface = app.world().resource::<SurfaceGrid>();
    let cover = app.world().resource::<CoverLedger>();
    let tuning = app.world().resource::<CombatTuning>();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the column x/y are tiny grid coords, exact in f32"
    )]
    let muzzle = SimPos::new(COL_X as f32 + 0.5, COL_Y as f32 + 0.5, 0.5);
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let result = march_vector(
        muzzle,
        dir,
        occupancy,
        surface,
        cover,
        tuning,
        CellLevel::new(column_cell(), Level::new(0)),
        |_| false,
    );
    matches!(result.kind, MarchKind::Slab)
}

/// Install a populated Torso injury table + registry so a non-graze torso wound rolls a
/// named injury (the QA(5) `InjuryInflicted` proof) — the SHARED (category, severity) pool,
/// used AS-IS (no falling-specific source dimension; GTW-452 owns that).
fn install_torso_injury_content(app: &mut App) {
    use gdtf_battle_sim::{
        InjuryCategory, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
        InspectText, LogText, PopupText, PostHeal, Severity, WeightedInjuryEntry,
        WeightedInjuryTable,
    };
    let name = InjuryName::new("bruised_ribs".to_owned());
    // A minimal named def in the registry (no effects — the roll just needs a resolvable key).
    let def = InjuryDef {
        name:         name.clone(),
        category:     InjuryCategory::Torso,
        severity:     Severity::Minor,
        popup_text:   PopupText::new("Bruised!".to_owned()),
        log_text:     LogText::new("bruised ribs".to_owned()),
        inspect_text: InspectText::new("Bruised ribs from the fall.".to_owned()),
        effects:      Vec::new(),
        post_heal:    PostHeal::Deferred,
    };
    app.insert_resource(InjuryRegistry::new([(name.clone(), def)]));
    // Weight every non-graze/non-fatal severity for the Torso category to this one key, so any
    // wound tier that lands on the torso rolls it.
    let table = || {
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
            name.clone(),
            InjuryWeight::new(1),
        )])
    };
    app.insert_resource(InjuryTables::new([
        ((InjuryCategory::Torso, Severity::Minor), table()),
        ((InjuryCategory::Torso, Severity::Major), table()),
        ((InjuryCategory::Torso, Severity::Critical), table()),
    ]));
}
